package dev.roncore.sdk.auth;

import java.time.Instant;
import java.util.Objects;
import java.util.function.Supplier;

/**
 * RO:WHAT —
 *   {@link TokenProvider} that refreshes tokens via a user-supplied callback.
 *
 * RO:WHY —
 *   Lets apps integrate with external auth systems (svc-passport, OAuth)
 *   without the SDK knowing details.
 *
 * RO:INVARIANTS —
 *   - Tokens are stored in memory only.
 *   - Minimal caching to avoid hammering upstream auth.
 */
public final class RefreshingTokenProvider implements TokenProvider {

    private final Supplier<String> refresher;
    private final long ttlSeconds;

    private volatile String cachedToken;
    private volatile Instant expiry;

    public RefreshingTokenProvider(Supplier<String> refresher, long ttlSeconds) {
        this.refresher = Objects.requireNonNull(refresher, "refresher must not be null");
        this.ttlSeconds = Math.max(0L, ttlSeconds);
    }

    @Override
    public String getToken() {
        if (ttlSeconds <= 0) {
            // No caching: always refresh.
            return refresher.get();
        }

        Instant now = Instant.now();
        String token = cachedToken;
        Instant exp = expiry;

        if (token != null && exp != null && now.isBefore(exp)) {
            return token;
        }

        synchronized (this) {
            token = cachedToken;
            exp = expiry;
            if (token != null && exp != null && now.isBefore(exp)) {
                return token;
            }

            token = refresher.get();
            cachedToken = token;
            expiry = now.plusSeconds(ttlSeconds);
            return token;
        }
    }
}
