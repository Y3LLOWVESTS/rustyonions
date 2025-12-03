package dev.roncore.sdk.internal;

import dev.roncore.sdk.RonException;

/**
 * RO:WHAT —
 *   Simple guard that enforces a maximum response size.
 *
 * RO:WHY —
 *   Reduces DoS risk when talking to misconfigured or hostile gateways.
 */
public final class ResponseSizeLimiter {

    private final long maxBytes;

    public ResponseSizeLimiter(long maxBytes) {
        this.maxBytes = maxBytes;
    }

    public void ensureWithinLimit(long actualBytes) {
        if (maxBytes <= 0) {
            return;
        }
        if (actualBytes > maxBytes) {
            throw RonException.transportError(
                    "RESPONSE_TOO_LARGE",
                    "Response exceeded configured size limit",
                    false,
                    null
            );
        }
    }
}
