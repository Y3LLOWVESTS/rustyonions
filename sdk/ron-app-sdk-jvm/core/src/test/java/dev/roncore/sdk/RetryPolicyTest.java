package dev.roncore.sdk;

import dev.roncore.sdk.internal.BackoffStrategy;
import dev.roncore.sdk.internal.RetryPolicy;
import org.junit.jupiter.api.Test;

import java.time.Duration;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Retry/backoff policy tests.
 *
 * RO:WHAT  — Tests for bounded retry behavior and exponential backoff.
 * RO:WHY   — Ensures we never spin infinite loops and that retries obey
 *            SDK_SECURITY / SDK_IDB invariants for bounded retries + jitter.
 * RO:INTERACTS —
 *   - RetryPolicy
 *   - BackoffStrategy
 * RO:INVARIANTS —
 *   - Non-idempotent requests do not retry by default.
 *   - Idempotent requests use bounded retries (maxRetries).
 *   - Backoff delays grow and are capped.
 */
public class RetryPolicyTest {

    @Test
    void nonIdempotentRequestsDoNotRetryByDefault() {
        RetryPolicy policy = RetryPolicy.defaultPolicy();

        // attempt is 1-based; simulate POST/DELETE (non-idempotent)
        boolean shouldRetry = policy.shouldRetry(
                1,
                500,      // server error
                false     // idempotent=false
        );

        assertFalse(shouldRetry, "Non-idempotent operations should not retry by default");
    }

    @Test
    void idempotentRequestsRetryUpToMaxRetries() {
        int maxRetries = 3;
        RetryPolicy policy = RetryPolicy.exponentialBackoff(
                maxRetries,
                Duration.ofMillis(100),
                Duration.ofSeconds(5),
                0.2 // jitter
        );

        // For an idempotent GET on transient errors, we should retry up to maxRetries
        for (int attempt = 1; attempt <= maxRetries; attempt++) {
            boolean shouldRetry = policy.shouldRetry(
                    attempt,
                    503,      // typical transient status
                    true      // idempotent=true
            );
            assertTrue(shouldRetry, "Should retry attempt " + attempt + " for transient idempotent request");
        }

        // Once we've hit maxRetries, the next attempt must not retry.
        boolean shouldRetryAfterMax = policy.shouldRetry(
                maxRetries + 1,
                503,
                true
        );
        assertFalse(shouldRetryAfterMax, "Should not retry after maxRetries has been reached");
    }

    @Test
    void backoffDelaysGrowAndAreCapped() {
        int maxRetries = 5;
        Duration base = Duration.ofMillis(100);
        Duration max = Duration.ofSeconds(2);

        BackoffStrategy strategy = BackoffStrategy.exponential(base, max, 0.0); // no jitter for deterministic test

        Duration d1 = strategy.nextDelay(1);
        Duration d2 = strategy.nextDelay(2);
        Duration d3 = strategy.nextDelay(3);
        // Use attempt number well beyond maxRetries to ensure we hit the cap.
        Duration dFar = strategy.nextDelay(maxRetries + 10);

        assertTrue(d2.compareTo(d1) > 0, "Second delay should be greater than first");
        assertTrue(d3.compareTo(d2) > 0, "Third delay should be greater than second");
        assertTrue(dFar.compareTo(max) <= 0, "Delay should be capped at max");
    }
}
