package dev.roncore.sdk.config;

import java.time.Duration;
import java.util.Map;

/**
 * RO:WHAT —
 *   Helpers that read {@code RON_SDK_*} env vars and apply sane defaults.
 *
 * RO:WHY —
 *   Mirrors TS SDK semantics on server/CLI: env provides defaults, explicit
 *   code config always wins. Android ignores env entirely.
 *
 * RO:INVARIANTS —
 *   - fromEnv() uses System.getenv() for convenience.
 *   - applyEnv(...) is deterministic and testable with a supplied map.
 */
public final class EnvConfigLoader {

    private EnvConfigLoader() {
    }

    /**
     * Convenience entrypoint that starts from an empty builder and
     * populates it from the process environment.
     */
    public static RonConfig.Builder fromEnv() {
        RonConfig.Builder builder = RonConfig.builder();
        applyEnv(builder, System.getenv());
        return builder;
    }

    /**
     * Applies environment-derived settings to an existing builder.
     *
     * Explicit builder values take precedence:
     *   - If baseUrl is already set on the builder, RON_SDK_GATEWAY_ADDR is ignored.
     */
    public static void applyEnv(RonConfig.Builder builder, Map<String, String> env) {
        if (env == null || env.isEmpty()) {
            return;
        }

        // Base URL
        String baseUrl = env.get("RON_SDK_GATEWAY_ADDR");
        if (!builder.hasBaseUrl() && baseUrl != null && !baseUrl.isBlank()) {
            builder.baseUrl(baseUrl.trim());
        }

        // Timeouts
        Long overallMs = parseMillis(env.get("RON_SDK_OVERALL_TIMEOUT_MS"));
        Long connectMs = parseMillis(env.get("RON_SDK_CONNECT_TIMEOUT_MS"));
        Long readMs = parseMillis(env.get("RON_SDK_READ_TIMEOUT_MS"));
        Long writeMs = parseMillis(env.get("RON_SDK_WRITE_TIMEOUT_MS"));

        if (overallMs != null) {
            builder.overallTimeoutMs(overallMs);
        } else {
            // default overall timeout if none is provided at all
            builder.overallTimeout(Duration.ofMillis(30_000L));
        }

        if (connectMs != null) {
            builder.connectTimeoutMs(connectMs);
        }
        if (readMs != null) {
            builder.readTimeoutMs(readMs);
        }
        if (writeMs != null) {
            builder.writeTimeoutMs(writeMs);
        }

        // Insecure HTTP flag
        String insecure = env.get("RON_SDK_INSECURE_HTTP");
        if (insecure != null) {
            boolean allow = "1".equals(insecure) || "true".equalsIgnoreCase(insecure);
            builder.allowInsecureHttp(allow);
        }
    }

    private static Long parseMillis(String value) {
        if (value == null) {
            return null;
        }
        try {
            long millis = Long.parseLong(value.trim());
            if (millis <= 0) {
                return null;
            }
            return millis;
        } catch (NumberFormatException ex) {
            return null;
        }
    }
}
