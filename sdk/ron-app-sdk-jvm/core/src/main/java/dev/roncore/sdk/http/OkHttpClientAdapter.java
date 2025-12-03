package dev.roncore.sdk.http;

import dev.roncore.sdk.RonException;
import dev.roncore.sdk.config.RonConfig;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import okhttp3.MediaType;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;

/**
 * RO:WHAT —
 *   Default {@link HttpClientAdapter} backed by OkHttp.
 *
 * RO:WHY —
 *   Provides a battle-tested HTTP client that works across JVM servers
 *   and Android, with connection pooling and timeouts.
 *
 * RO:INVARIANTS —
 *   - Uses timeouts from {@link RonConfig}.
 *   - Does not log or expose tokens.
 */
public final class OkHttpClientAdapter implements HttpClientAdapter {

    private static final MediaType JSON = MediaType.get("application/json; charset=utf-8");

    private final OkHttpClient client;

    public OkHttpClientAdapter(RonConfig config) {
        this.client = new OkHttpClient.Builder()
                .connectTimeout(config.getConnectTimeout())
                .readTimeout(config.getReadTimeout())
                .writeTimeout(config.getWriteTimeout())
                .build();
    }

    @Override
    public HttpResponse execute(HttpRequestContext requestContext) throws IOException {
        Request.Builder builder = new Request.Builder()
                .url(requestContext.getUrl());

        for (Map.Entry<String, String> header : requestContext.getHeaders().entrySet()) {
            builder.header(header.getKey(), header.getValue());
        }

        String method = requestContext.getMethod();
        String body = requestContext.getBody();

        if ("GET".equalsIgnoreCase(method) || "DELETE".equalsIgnoreCase(method)) {
            builder.method(method, null);
        } else {
            RequestBody requestBody = body == null
                    ? RequestBody.create(new byte[0], JSON)
                    : RequestBody.create(body, JSON);
            builder.method(method, requestBody);
        }

        try (Response response = client.newCall(builder.build()).execute()) {
            int statusCode = response.code();
            String responseBody = response.body() != null ? response.body().string() : null;

            Map<String, List<String>> headers = new HashMap<>();
            for (String name : response.headers().names()) {
                headers.put(name, new ArrayList<>(response.headers(name)));
            }

            return new HttpResponse(statusCode, responseBody, headers);
        } catch (IOException ex) {
            // Transport-level failure, mapped to RonException at a higher layer.
            throw ex;
        } catch (RuntimeException ex) {
            throw RonException.transportError("HTTP_CLIENT_ERROR", "HTTP client error", false, ex);
        }
    }
}
