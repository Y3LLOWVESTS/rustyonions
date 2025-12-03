package dev.roncore.sdk;

/**
 * RO:WHAT —
 *   Holds SDK and protocol version constants for logging and diagnostics.
 *
 * RO:WHY —
 *   Allows apps and support tooling to quickly identify which SDK and
 *   protocol version were used when making a request.
 *
 * RO:INVARIANTS —
 *   - Values are updated as part of release process; never mutated at runtime.
 */
public final class RonSdkVersion {

    public static final String SDK_VERSION = "0.1.0-SNAPSHOT";
    public static final String PROTOCOL_VERSION = "v0.1";

    private RonSdkVersion() {
    }
}
