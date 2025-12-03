package dev.roncore.sdk.auth;

/**
 * RO:WHAT —
 *   Functional interface for providing capability/macroon tokens on demand.
 *
 * RO:WHY —
 *   Allows JVM apps to centralize token rotation and storage instead of
 *   scattering Authorization headers across code.
 *
 * RO:INVARIANTS —
 *   - Implementations must keep tokens in memory only (no disk persistence).
 *   - SDK consumers must ensure least-privilege and rotation.
 */
@FunctionalInterface
public interface TokenProvider {

    /**
     * Returns the current token/macroon, or {@code null} if no token is available.
     *
     * Implementations must not log or otherwise leak the token.
     */
    String getToken();
}
