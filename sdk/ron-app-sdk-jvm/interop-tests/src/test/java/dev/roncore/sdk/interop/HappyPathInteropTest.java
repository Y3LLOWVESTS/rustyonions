package dev.roncore.sdk.interop;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonProblem;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * RO:WHAT —
 *   "Happy path" interop test that expects a successful envelope from
 *   the canonical `/app/ping` route via the JVM SDK.
 *
 * RO:WHY —
 *   - Complements {@link InteropSmokeTest} (which is intentionally
 *     tolerant of early 404s / bare errors for dev-preview).
 *   - This test is for environments where we KNOW `/app/ping` is
 *     wired up and should return a success envelope (e.g. stable
 *     macronode profile or CI interop node).
 *
 * RO:INVARIANTS —
 *   - If `RON_SDK_GATEWAY_ADDR` is not set, the test is SKIPPED.
 *   - If `RON_SDK_INTEROP_EXPECT_PING_OK` is not "1", the test is SKIPPED.
 *   - When enabled, the test asserts:
 *       * HTTP status is 2xx.
 *       * `ok()` is true.
 *       * `problem` is null.
 *       * Data envelope is present (may be empty map).
 */
public class HappyPathInteropTest {

    @Test
    void pingReturnsOkWhenHappyPathIsEnabled() {
        String gatewayAddr = System.getenv("RON_SDK_GATEWAY_ADDR");
        Assumptions.assumeTrue(
                gatewayAddr != null && !gatewayAddr.isBlank(),
                "RON_SDK_GATEWAY_ADDR is not set; skipping happy-path interop test."
        );

        String expectOk = System.getenv("RON_SDK_INTEROP_EXPECT_PING_OK");
        Assumptions.assumeTrue(
                "1".equals(expectOk),
                "RON_SDK_INTEROP_EXPECT_PING_OK != 1; skipping strict happy-path test."
        );

        try (RonClient client = RonClient.builder()
                .fromEnv()
                .build()) {

            @SuppressWarnings("unchecked")
            AppResponse<Map<String, Object>> response =
                    client.get("/ping", (Class<Map<String, Object>>) (Class<?>) Map.class);

            assertNotNull(response, "AppResponse must not be null");

            int status = response.getStatus();
            assertTrue(
                    status >= 200 && status < 300,
                    "Expected 2xx status from /app/ping, got " + status
            );

            assertTrue(response.ok(), "Expected response.ok() to be true for /app/ping");

            RonProblem problem = response.getProblem();
            assertNull(problem, "Expected no problem envelope on happy-path /app/ping");

            Map<String, Object> data = response.getData();
            assertNotNull(data, "Expected non-null data for happy-path /app/ping");
        }
    }
}
