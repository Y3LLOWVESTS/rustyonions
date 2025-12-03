package dev.roncore.sdk.http;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

/**
 * RO:WHAT —
 *   Immutable description of an HTTP request to the gateway.
 *
 * RO:WHY —
 *   Decouples {@code RonClient} from the concrete HTTP engine while
 *   keeping enough context for logging and backoff decisions.
 */
public final class HttpRequestContext {

    private final String method;
    private final String url;
    private final Map<String, String> headers;
    private final String body; // JSON or null

    public HttpRequestContext(String method, String url, Map<String, String> headers, String body) {
        this.method = method;
        this.url = url;
        this.headers = headers != null ? Collections.unmodifiableMap(new HashMap<>(headers)) : Collections.emptyMap();
        this.body = body;
    }

    public String getMethod() {
        return method;
    }

    public String getUrl() {
        return url;
    }

    public Map<String, String> getHeaders() {
        return headers;
    }

    public String getBody() {
        return body;
    }
}
