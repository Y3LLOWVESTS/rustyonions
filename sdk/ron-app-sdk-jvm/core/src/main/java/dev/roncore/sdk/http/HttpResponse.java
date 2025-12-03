package dev.roncore.sdk.http;

import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * RO:WHAT —
 *   Minimal HTTP response representation returned by HTTP adapters.
 *
 * RO:WHY —
 *   Keeps transport details out of higher layers while still exposing
 *   status, headers, and body for envelope parsing.
 */
public final class HttpResponse {

    private final int statusCode;
    private final String body;
    private final Map<String, List<String>> headers;

    public HttpResponse(int statusCode, String body, Map<String, List<String>> headers) {
        this.statusCode = statusCode;
        this.body = body;
        this.headers = headers != null ? Collections.unmodifiableMap(new HashMap<>(headers)) : Collections.emptyMap();
    }

    public int getStatusCode() {
        return statusCode;
    }

    public String getBody() {
        return body;
    }

    public Map<String, List<String>> getHeaders() {
        return headers;
    }
}
