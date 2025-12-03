package dev.roncore.sdk.auth;

import java.util.Objects;

/**
 * RO:WHAT —
 *   Simple in-memory {@link TokenProvider} for tests and small tools.
 *
 * RO:WHY —
 *   Provides a straightforward way to inject a single token without
 *   introducing global statics or disk persistence.
 */
public final class StaticTokenProvider implements TokenProvider {

    private final String token;

    public StaticTokenProvider(String token) {
        this.token = Objects.requireNonNull(token, "token must not be null");
    }

    @Override
    public String getToken() {
        return token;
    }
}
