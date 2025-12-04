package dev.roncore.sdk.interop;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;
import dev.roncore.sdk.RonProblem;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * RO:WHAT —
 *   Interop test that verifies how the JVM SDK maps error responses
 *   from a live RON-CORE Micronode/Macronode gateway into AppResponse /
 *   RonProblem for a deliberately failing route.
 *
 * RO:WHY —
 *   Happy-path interop is covered by {@link HappyPathInteropTest}, but we also
 *   want to sanity check that:
 *     - HTTP errors come back with a 4xx/5xx status code, and
 *     - AppResponse#ok() is false for those errors.
 *
 *   If the gateway already returns a structured error envelope (AppResponse
 *   with ok=false + problem), we also lightly validate the RonProblem fields,
 *   but we do NOT require that yet.
 *
 * RO:INVARIANTS —
 *   - MUST be safe to skip when no gateway is available.
 *   - MUST only run when the caller explicitly opts in via
 *     RON_SDK_INTEROP_EXPECT_ERROR_ENVELOPE=1.
 *   - WHEN enabled, MUST:
 *       * receive an AppResponse with ok=false,
 *       * see an HTTP status in the 4xx–5xx range,
 *       * and, if a RonProblem is present, it must have a non-blank message.
 */
class ErrorEnvelopeInteropTest {

    private static final String GATEWAY_ENV = "RON_SDK_GATEWAY_ADDR";
    private static final String EXPECT_ERROR_ENV = "RON_SDK_INTEROP_EXPECT_ERROR_ENVELOPE";

    @Test
    void errorEnvelopeShouldPopulateRonProblemWhenPresent() throws RonException {
        String gatewayAddr = System.getenv(GATEWAY_ENV);
        String expectErrorEnvelope = System.getenv(EXPECT_ERROR_ENV);

        // 1) Skip cleanly if we don't have a gateway configured.
        Assumptions.assumeTrue(
                gatewayAddr != null && !gatewayAddr.isBlank(),
                () -> GATEWAY_ENV + " must be set to run error-envelope interop tests"
        );

        // 2) Skip unless the caller explicitly opts in to running this test.
        Assumptions.assumeTrue(
                "1".equals(expectErrorEnvelope),
                () -> EXPECT_ERROR_ENV + "=1 must be set to run error-envelope interop tests"
        );

        // 3) Build a RonClient from env, same as other interop tests.
        RonClient client = RonClient.builder()
                .fromEnv()
                .build();

        // 4) Call a deliberately invalid path. Once we stabilize a canonical
        //    "force error" route on the gateway, this path can be updated.
        @SuppressWarnings("rawtypes")
        AppResponse<Map> response = client.get("/does-not-exist", Map.class);

        assertNotNull(response, "Response must not be null");

        int status = response.getStatus();
        RonProblem problem = response.getProblem();

        String debugSummary = "status=" + status
                + ", ok=" + response.ok()
                + ", problem=" + (problem == null
                    ? "null"
                    : ("code=" + problem.getCode() + ", message=" + problem.getMessage()));

        // 5) Baseline expectations: this MUST be treated as an error.
        assertTrue(
                status >= 400 && status <= 599,
                () -> "Expected HTTP error status (4xx/5xx). " + debugSummary
        );

        assertFalse(
                response.ok(),
                () -> "Error route must set ok=false on the AppResponse. " + debugSummary
        );

        // 6) If the gateway already sends a structured RonProblem, lightly validate it.
        if (problem != null) {
            String message = problem.getMessage();
            assertNotNull(message, () -> "RonProblem.message must not be null. " + debugSummary);
            assertFalse(
                    message.isBlank(),
                    () -> "RonProblem.message must not be blank. " + debugSummary
            );

            // Code is nice to have but we don't hard-fail if it's blank yet; that
            // can be tightened once the gateway's error envelope is fully standardized.
            String code = problem.getCode();
            if (code != null) {
                assertFalse(
                        code.isBlank(),
                        () -> "RonProblem.code, if present, must not be blank. " + debugSummary
                );
            }
        }
    }
}
