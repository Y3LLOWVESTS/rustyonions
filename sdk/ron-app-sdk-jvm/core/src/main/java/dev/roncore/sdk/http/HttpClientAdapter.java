package dev.roncore.sdk.http;

import java.io.IOException;

/**
 * RO:WHAT —
 *   Internal abstraction for HTTP operations used by {@code RonClient}.
 *
 * RO:WHY —
 *   Allows swapping HTTP engines (OkHttp, Ktor, custom) without changing
 *   the public SDK surface.
 *
 * RO:INVARIANTS —
 *   - All calls are bounded by timeouts supplied via config.
 *   - Implementations never log secrets (headers/tokens).
 */
public interface HttpClientAdapter {

    HttpResponse execute(HttpRequestContext request) throws IOException;
}
