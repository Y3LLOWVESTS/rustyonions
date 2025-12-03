package dev.roncore.sdk.internal;

import dev.roncore.sdk.RonException;

import java.time.Duration;
import java.util.Objects;
import java.util.concurrent.Callable;

/**
 * RO:WHAT —
 *   Encapsulates retry behavior for idempotent requests.
 *
 * RO:WHY —
 *   Ensures all retries are bounded and obey SDK invariants about
 *   idempotency and backoff.
 */
public final class RetryPolicy {

    private final int maxRetries;
    private final BackoffStrategy backoff;

    public RetryPolicy(int maxRetries, BackoffStrategy backoff) {
        this.maxRetries = Math.max(0, maxRetries);
        this.backoff = Objects.requireNonNull(backoff, "backoff must not be null");
    }

    /**
     * SDK default: no retries.
     */
    public static RetryPolicy defaultPolicy() {
        return new RetryPolicy(0, BackoffStrategy.noBackoff());
    }

    /**
     * Factory for exponential backoff retry policy.
     */
    public static RetryPolicy exponentialBackoff(
            int maxRetries,
            Duration baseDelay,
            Duration maxDelay,
            double jitterFraction
    ) {
        return new RetryPolicy(maxRetries, BackoffStrategy.exponential(baseDelay, maxDelay, jitterFraction));
    }

    /**
     * Predicate used by tests and by execute(): should we retry this attempt?
     *
     * @param attempt     1-based attempt number
     * @param statusCode  HTTP status code (approximation; may be 503 for transport errors)
     * @param idempotent  whether the operation is idempotent (GET/HEAD)
     */
    public boolean shouldRetry(int attempt, int statusCode, boolean idempotent) {
        if (!idempotent) {
            return false;
        }
        if (attempt > maxRetries) {
            return false;
        }

        // Typical transient codes: 408, 429, 5xx
        if (statusCode == 408 || statusCode == 429) {
            return true;
        }
        return statusCode >= 500 && statusCode < 600;
    }

    /**
     * Execute a callable with retry behavior applied based on HTTP method.
     * This is what {@link dev.roncore.sdk.RonClient} uses.
     */
    public <T> T execute(String method, Callable<T> callable) {
        boolean idempotent = isIdempotent(method);
        int attempt = 0;

        while (true) {
            try {
                return callable.call();
            } catch (RonException ex) {
                // Already structured; if retryable and idempotent, fall through
                attempt++;
                if (!idempotent || !ex.isRetryable() || attempt > maxRetries) {
                    throw ex;
                }
                sleep(backoff.nextDelay(attempt));
            } catch (Exception ex) {
                // Transport / unexpected; approximate status as 503.
                attempt++;
                boolean canRetry = shouldRetry(attempt, 503, idempotent);
                if (!canRetry) {
                    throw RonException.transportError(
                            "TRANSPORT_ERROR",
                            "Transport error during request",
                            idempotent,
                            ex
                    );
                }
                sleep(backoff.nextDelay(attempt));
            }
        }
    }

    private static boolean isIdempotent(String method) {
        if (method == null) {
            return false;
        }
        String m = method.toUpperCase();
        return "GET".equals(m) || "HEAD".equals(m);
    }

    private static void sleep(Duration d) {
        long millis = d.toMillis();
        if (millis <= 0) {
            return;
        }
        try {
            Thread.sleep(millis);
        } catch (InterruptedException ie) {
            Thread.currentThread().interrupt();
            throw RonException.transportError(
                    "RETRY_INTERRUPTED",
                    "Retry interrupted",
                    false,
                    ie
            );
        }
    }
}
