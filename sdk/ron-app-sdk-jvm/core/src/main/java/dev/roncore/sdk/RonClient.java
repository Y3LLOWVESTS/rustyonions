package dev.roncore.sdk;

import dev.roncore.sdk.auth.TokenProvider;
import dev.roncore.sdk.config.EnvConfigLoader;
import dev.roncore.sdk.config.RonConfig;
import dev.roncore.sdk.http.HttpClientAdapter;
import dev.roncore.sdk.http.HttpRequestContext;
import dev.roncore.sdk.http.HttpResponse;
import dev.roncore.sdk.http.OkHttpClientAdapter;
import dev.roncore.sdk.internal.JsonMapper;
import dev.roncore.sdk.internal.ResponseSizeLimiter;
import dev.roncore.sdk.internal.RetryPolicy;

import java.io.Closeable;
import java.io.IOException;
import java.net.URI;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;

/**
 * RO:WHAT —
 *   Java-first RON client for JVM apps (Spring Boot, CLIs, desktop).
 *
 * RO:WHY —
 *   Provides an idiomatic API for calling `/app/*` endpoints with
 *   env-driven defaults, structured errors, and safety invariants.
 *
 * RO:INTERACTS —
 *   - {@link RonConfig} (configuration).
 *   - {@link HttpClientAdapter} (transport).
 *   - {@link TokenProvider} (auth).
 *   - {@link AppResponse} / {@link RonProblem} (envelopes).
 *
 * RO:INVARIANTS —
 *   - Thread-safe and intended to be shared per process or per DI scope.
 *   - All network calls are bounded by timeouts and optional retries.
 */
public final class RonClient implements Closeable {

    private final RonConfig config;
    private final HttpClientAdapter http;
    private final TokenProvider tokenProvider;
    private final JsonMapper jsonMapper;
    private final RetryPolicy retryPolicy;
    private final ResponseSizeLimiter responseSizeLimiter;

    private RonClient(Builder builder) {
        this.config = builder.config;
        this.http = builder.httpClientAdapter != null
                ? builder.httpClientAdapter
                : new OkHttpClientAdapter(config);
        this.tokenProvider = builder.tokenProvider;
        this.jsonMapper = builder.jsonMapper != null ? builder.jsonMapper : new JsonMapper();
        this.retryPolicy = builder.retryPolicy != null ? builder.retryPolicy : RetryPolicy.defaultPolicy();
        this.responseSizeLimiter = new ResponseSizeLimiter(config.getMaxResponseBytes());
    }

    public static Builder builder() {
        return new Builder();
    }

    /**
     * Performs a GET request to {@code /app/*}, returning an {@link AppResponse}.
     */
    public AppResponse<String> get(String path) {
        return get(path, String.class);
    }

    public <T> AppResponse<T> get(String path, Class<T> dataType) {
        return execute("GET", path, null, dataType);
    }

    public <T> AppResponse<T> post(String path, Object body, Class<T> dataType) {
        return execute("POST", path, body, dataType);
    }

    public <T> AppResponse<T> put(String path, Object body, Class<T> dataType) {
        return execute("PUT", path, body, dataType);
    }

    public <T> AppResponse<T> patch(String path, Object body, Class<T> dataType) {
        return execute("PATCH", path, body, dataType);
    }

    public <T> AppResponse<T> delete(String path, Class<T> dataType) {
        return execute("DELETE", path, null, dataType);
    }

    private <T> AppResponse<T> execute(String method, String path, Object body, Class<T> dataType) {
        Objects.requireNonNull(method, "method must not be null");
        Objects.requireNonNull(path, "path must not be null");

        final String url = buildUrl(path);
        final Map<String, String> headers = buildHeaders();

        final String jsonBody = jsonMapper.toJson(body);

        HttpRequestContext ctx = new HttpRequestContext(method, url, headers, jsonBody);

        try {
            HttpResponse httpResponse = retryPolicy.execute(method, () -> {
                HttpResponse response;
                try {
                    response = http.execute(ctx);
                } catch (IOException ex) {
                    throw RonException.transportError(
                            "TRANSPORT_IO_ERROR",
                            "I/O error during HTTP call",
                            true,
                            ex
                    );
                }

                String responseBody = response.getBody();
                if (responseBody != null) {
                    responseSizeLimiter.ensureWithinLimit(responseBody.length());
                }
                return response;
            });

            return jsonMapper.decodeAppResponse(httpResponse, dataType);
        } catch (RonException ex) {
            // Already structured; just bubble up.
            throw ex;
        } catch (RuntimeException ex) {
            throw RonException.transportError(
                    "UNEXPECTED_RUNTIME_ERROR",
                    "Unexpected error during request",
                    false,
                    ex
            );
        }
    }

    private String buildUrl(String path) {
        URI base = config.getBaseUri();
        String normalizedPath = path.startsWith("/app/") ? path : "/app" + (path.startsWith("/") ? path : "/" + path);
        String baseStr = base.toString();
        if (baseStr.endsWith("/") && normalizedPath.startsWith("/")) {
            return baseStr.substring(0, baseStr.length() - 1) + normalizedPath;
        }
        return baseStr + normalizedPath;
    }

    private Map<String, String> buildHeaders() {
        Map<String, String> headers = new HashMap<>();
        headers.put("User-Agent", "ron-app-sdk-jvm/" + RonSdkVersion.SDK_VERSION);
        headers.put("X-Ron-Protocol-Version", RonSdkVersion.PROTOCOL_VERSION);

        if (tokenProvider != null) {
            String token = tokenProvider.getToken();
            if (token != null && !token.isBlank()) {
                headers.put("Authorization", "Bearer " + token);
            }
        }

        return headers;
    }

    @Override
    public void close() {
        // OkHttp client does not strictly require close; we may expose a hook later
        // for shutting down connection pools or other resources.
    }

    /**
     * Builder for {@link RonClient}.
     */
    public static final class Builder {

        private RonConfig config;
        private RonConfig.Builder configBuilder;
        private HttpClientAdapter httpClientAdapter;
        private TokenProvider tokenProvider;
        private JsonMapper jsonMapper;
        private RetryPolicy retryPolicy;

        /**
         * Load configuration defaults from {@code RON_SDK_*} env vars.
         * Explicit builder calls can override these values via the builder.
         */
        public Builder fromEnv() {
            this.configBuilder = EnvConfigLoader.fromEnv();
            this.config = null;
            return this;
        }

        /**
         * Explicitly sets the base URL; overrides any env-derived base URL.
         * If no config has been set yet, starts from an empty RonConfig builder.
         */
        public Builder baseUrl(String baseUrl) {
            if (this.configBuilder == null) {
                this.configBuilder = RonConfig.builder();
            }
            this.configBuilder.baseUrl(baseUrl);
            this.config = null;
            return this;
        }

        /**
         * Use a fully-built {@link RonConfig}, bypassing env and builder state.
         */
        public Builder config(RonConfig config) {
            this.config = Objects.requireNonNull(config, "config must not be null");
            this.configBuilder = null;
            return this;
        }

        public Builder tokenProvider(TokenProvider tokenProvider) {
            this.tokenProvider = tokenProvider;
            return this;
        }

        public Builder httpClientAdapter(HttpClientAdapter httpClientAdapter) {
            this.httpClientAdapter = httpClientAdapter;
            return this;
        }

        public Builder jsonMapper(JsonMapper jsonMapper) {
            this.jsonMapper = jsonMapper;
            return this;
        }

        public Builder retryPolicy(RetryPolicy retryPolicy) {
            this.retryPolicy = retryPolicy;
            return this;
        }

        public RonClient build() {
            if (this.config == null) {
                if (this.configBuilder == null) {
                    throw RonException.configError(
                            "CONFIG_MISSING_BASE_URL",
                            "No config or baseUrl provided; call fromEnv(), config(), or baseUrl()."
                    );
                }
                this.config = this.configBuilder.build();
            }
            return new RonClient(this);
        }
    }
}
