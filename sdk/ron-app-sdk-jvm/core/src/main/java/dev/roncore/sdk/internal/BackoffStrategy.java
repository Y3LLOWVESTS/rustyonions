package dev.roncore.sdk.internal;

import java.time.Duration;
import java.util.concurrent.ThreadLocalRandom;

/**
 * RO:WHAT —
 *   Backoff strategy for retries (exponential with optional jitter).
 *
 * RO:WHY —
 *   Provides bounded, jittered delays to avoid thundering herds when
 *   multiple clients retry idempotent requests.
 */
public interface BackoffStrategy {

    /**
     * Returns the delay for the given 1-based attempt number.
     */
    Duration nextDelay(int attempt);

    static BackoffStrategy noBackoff() {
        return attempt -> Duration.ZERO;
    }

    /**
     * Exponential backoff with optional jitter.
     *
     * @param base   base delay (first attempt).
     * @param max    maximum delay cap.
     * @param jitter jitter fraction [0.0, 1.0]; 0.0 = no jitter, 1.0 = full jitter.
     */
    static BackoffStrategy exponential(Duration base, Duration max, double jitter) {
        final long baseMs = base.toMillis();
        final long maxMs = Math.max(1L, max.toMillis());
        final double j = Math.max(0.0, Math.min(1.0, jitter));

        return attempt -> {
            int a = Math.max(1, attempt);
            long exp = baseMs * (1L << Math.min(a - 1, 10)); // cap exponent growth
            long capped = Math.min(exp, maxMs);

            if (j == 0.0) {
                return Duration.ofMillis(capped);
            }

            long jitterRange = (long) (capped * j);
            long min = capped - jitterRange;
            long maxVal = capped + jitterRange;
            long chosen = ThreadLocalRandom.current().nextLong(min, maxVal + 1);
            if (chosen < 0) {
                chosen = 0;
            }
            return Duration.ofMillis(chosen);
        };
    }
}
