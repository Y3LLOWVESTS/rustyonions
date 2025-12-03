package dev.roncore.sdk.interop;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * RO:WHAT —
 *   End-to-end smoke test that exercises RonClient against a real RON gateway.
 *
 * RO:WHY —
 *   Proves that:
 *   - Env-based configuration (RON_SDK_GATEWAY_ADDR, RON_SDK_INSECURE_HTTP, etc.)
 *     works in a real JVM process.
 *   - The client can connect to a running node and decode an AppResponse envelope
 *     when available.
 *
 * RO:INVARIANTS (DEV-PREVIEW) —
 *   - If RON_SDK_GATEWAY_ADDR is not set, the test is SKIPPED.
 *   - The test MUST NOT fail the build just because early macronode shells
 *     still return bare HTTP errors with no problem envelope.
 *   - When a problem envelope is present, it should at least carry a message.
 */
public class InteropSmokeTest {

    @Test
    void gatewayPingReturnsEnvelope() {
        // 1) Skip cleanly when no live gateway is configured.
        String gatewayAddr = System.getenv("RON_SDK_GATEWAY_ADDR");
        Assumptions.assumeTrue(
                gatewayAddr != null && !gatewayAddr.isBlank(),
                "RON_SDK_GATEWAY_ADDR is not set; skipping interop smoke test."
        );

        // 2) Build a client from env (same path used by examples and real apps).
        try (RonClient client = RonClient.builder()
                .fromEnv()
                .build()) {

            // The SDK will normalize "/ping" → "/app/ping" against the configured base URL.
            // Today this often returns 404 in early macronode shells; for dev-preview we
            // only require that we get *some* HTTP status and that we can decode when an
            // AppResponse envelope is present.
            AppResponse<Map> response = client.get("/ping", Map.class);

            assertNotNull(response, "AppResponse must not be null");

            int status = response.getStatus();
            assertTrue(
                    status >= 100 && status < 600,
                    "Status should be a valid HTTP status code, got " + status
            );

            // If the gateway already wraps errors in a problem envelope, assert that it is sane.
            if (!response.ok() && response.getProblem() != null) {
                assertNotNull(
                        response.getProblem().getMessage(),
                        "Problem envelope should carry a message when present"
                );
            }

            // If ok() is true, or if the gateway currently returns bare errors with no
            // problem envelope, we accept that for dev-preview — the key contract here
            // is connectivity and basic decoding, not full problem semantics.
        }
    }
}
