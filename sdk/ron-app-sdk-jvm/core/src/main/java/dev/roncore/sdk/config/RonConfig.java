package dev.roncore.sdk.config;

import dev.roncore.sdk.RonException;

import java.net.URI;
import java.net.URISyntaxException;
import java.time.Duration;
import java.util.Objects;

/**
 * RO:WHAT —
 *   Immutable configuration for {@code RonClient}: base URL, timeouts,
 *   retry/backoff options, and security-sensitive flags.
 *
 * RO:WHY —
 *   Centralizes config so defaults and env-derived values are consistent
 *   across JVM apps (server, desktop, CLI) while remaining safe for Android.
 *
 * RO:INTERACTS —
 *   - {@link dev.roncore.sdk.RonClient} consumes this config.
 *   - {@link EnvConfigLoader} populates it from {@code RON_SDK_*} env vars.
 *
 * RO:INVARIANTS —
 *   - HTTPS is required by default; plain HTTP must be explicitly allowed.
 *   - All timeouts are bounded and non-negative.
 */
public final class RonConfig {

    private final String baseUrl;   // string form (for tests / diagnostics)
    private final URI baseUri;      // parsed form (for RonClient)
    private final Duration connectTimeout;
    private final Duration readTimeout;
    private final Duration writeTimeout;
    private final Duration overallTimeout;
    private final boolean allowInsecureHttp;
    private final int maxRetries;
    private final long maxResponseBytes;

    private RonConfig(Builder builder) {
        this.baseUrl = builder.baseUrl;
        this.baseUri = builder.baseUri;
        this.connectTimeout = builder.connectTimeout;
        this.readTimeout = builder.readTimeout;
        this.writeTimeout = builder.writeTimeout;
        this.overallTimeout = builder.overallTimeout;
        this.allowInsecureHttp = builder.allowInsecureHttp;
        this.maxRetries = builder.maxRetries;
        this.maxResponseBytes = builder.maxResponseBytes;
    }

    /**
     * Human-friendly string representation of the base URL, matching
     * what the builder/env provided.
     */
    public String getBaseUrl() {
        return baseUrl;
    }

    /**
     * Parsed URI form, used internally by the client to concatenate paths.
     */
    public URI getBaseUri() {
        return baseUri;
    }

    public Duration getConnectTimeout() {
        return connectTimeout;
    }

    public Duration getReadTimeout() {
        return readTimeout;
    }

    public Duration getWriteTimeout() {
        return writeTimeout;
    }

    public Duration getOverallTimeout() {
        return overallTimeout;
    }

    public boolean isAllowInsecureHttp() {
        return allowInsecureHttp;
    }

    public int getMaxRetries() {
        return maxRetries;
    }

    public long getMaxResponseBytes() {
        return maxResponseBytes;
    }

    // Convenience getters in milliseconds for tests / diagnostics

    public long getConnectTimeoutMs() {
        return connectTimeout.toMillis();
    }

    public long getReadTimeoutMs() {
        return readTimeout.toMillis();
    }

    public long getWriteTimeoutMs() {
        return writeTimeout.toMillis();
    }

    public long getOverallTimeoutMs() {
        return overallTimeout.toMillis();
    }

    public static Builder builder() {
        return new Builder();
    }

    public static final class Builder {

        String baseUrl;          // raw string
        URI baseUri;             // parsed form
        Duration connectTimeout = Duration.ofSeconds(5);
        Duration readTimeout = Duration.ofSeconds(30);
        Duration writeTimeout = Duration.ofSeconds(30);
        Duration overallTimeout = Duration.ofSeconds(30);
        boolean allowInsecureHttp = false;
        int maxRetries = 0;
        long maxResponseBytes = 5L * 1024L * 1024L; // 5 MiB default

        public Builder baseUrl(String url) {
            Objects.requireNonNull(url, "baseUrl must not be null");
            String trimmed = url.trim();
            this.baseUrl = trimmed;
            try {
                this.baseUri = new URI(trimmed);
            } catch (URISyntaxException e) {
                throw RonException.configError(
                        "CONFIG_INVALID_URL",
                        "Invalid baseUrl: " + e.getMessage()
                );
            }
            return this;
        }

        public Builder connectTimeout(Duration timeout) {
            this.connectTimeout = Objects.requireNonNull(timeout);
            return this;
        }

        public Builder readTimeout(Duration timeout) {
            this.readTimeout = Objects.requireNonNull(timeout);
            return this;
        }

        public Builder writeTimeout(Duration timeout) {
            this.writeTimeout = Objects.requireNonNull(timeout);
            return this;
        }

        public Builder overallTimeout(Duration timeout) {
            this.overallTimeout = Objects.requireNonNull(timeout);
            return this;
        }

        // Millisecond-based helpers used by tests and env loader

        public Builder connectTimeoutMs(long millis) {
            if (millis > 0) {
                this.connectTimeout = Duration.ofMillis(millis);
            }
            return this;
        }

        public Builder readTimeoutMs(long millis) {
            if (millis > 0) {
                this.readTimeout = Duration.ofMillis(millis);
            }
            return this;
        }

        public Builder writeTimeoutMs(long millis) {
            if (millis > 0) {
                this.writeTimeout = Duration.ofMillis(millis);
            }
            return this;
        }

        public Builder overallTimeoutMs(long millis) {
            if (millis > 0) {
                this.overallTimeout = Duration.ofMillis(millis);
            }
            return this;
        }

        public Builder allowInsecureHttp(boolean allowInsecureHttp) {
            this.allowInsecureHttp = allowInsecureHttp;
            return this;
        }

        public Builder maxRetries(int maxRetries) {
            this.maxRetries = Math.max(0, maxRetries);
            return this;
        }

        public Builder maxResponseBytes(long maxResponseBytes) {
            this.maxResponseBytes = maxResponseBytes;
            return this;
        }

        boolean hasBaseUrl() {
            return this.baseUrl != null;
        }

        public RonConfig build() {
            if (baseUrl == null || baseUrl.isBlank()) {
                throw RonException.configError(
                        "CONFIG_MISSING_BASE_URL",
                        "baseUrl is required (set RON_SDK_GATEWAY_ADDR or call baseUrl())."
                );
            }

            // Ensure baseUri is in sync with baseUrl (in case someone bypassed baseUrl())
            if (baseUri == null) {
                try {
                    baseUri = new URI(baseUrl);
                } catch (URISyntaxException e) {
                    throw RonException.configError(
                            "CONFIG_INVALID_URL",
                            "Invalid baseUrl: " + e.getMessage()
                    );
                }
            }

            String scheme = baseUri.getScheme();
            if (!"https".equalsIgnoreCase(scheme) && !allowInsecureHttp) {
                throw RonException.configError(
                        "CONFIG_INSECURE_HTTP_DISABLED",
                        "Plain HTTP is disabled by default; enable allowInsecureHttp(true) for dev/test only."
                );
            }

            return new RonConfig(this);
        }
    }
}
