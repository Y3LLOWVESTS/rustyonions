package dev.roncore.sdk;

import org.junit.jupiter.api.Test;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for the canonical error / response envelope types.
 *
 * RO:WHAT  — Validates AppResponse<T> and RonProblem wiring.
 * RO:WHY   — RON-CORE uses a canonical error envelope; the JVM SDK must
 *            reflect that shape faithfully for both languages.
 * RO:INTERACTS —
 *   - AppResponse<T>
 *   - RonProblem
 * RO:INVARIANTS —
 *   - Successful responses expose data + ok=true, problem=null.
 *   - Error responses expose problem + ok=false, data=null (for most cases).
 */
public class ErrorParsingTest {

    @Test
    void appResponseOkShouldExposeDataAndMarkOk() {
        AppResponse<String> response = AppResponse.ok("hello", 200);

        assertTrue(response.ok(), "ok() should be true for successful responses");
        assertEquals(200, response.getStatus());
        assertEquals("hello", response.getData());
        assertNull(response.getProblem(), "problem should be null on success");
    }

    @Test
    void appResponseErrorShouldExposeProblemAndMarkNotOk() {
        Map<String, Object> details = new HashMap<>();
        details.put("hint", "check token");

        RonProblem problem = new RonProblem(
                "AUTH_UNAUTHORIZED",
                "Unauthorized",
                "auth",
                "corr-123",
                details
        );

        AppResponse<Void> response = AppResponse.error(problem, 401);

        assertFalse(response.ok(), "ok() should be false when problem is present");
        assertEquals(401, response.getStatus());
        assertNull(response.getData(), "data should be null on error AppResponse");
        assertNotNull(response.getProblem(), "problem should not be null on error AppResponse");
        assertEquals("AUTH_UNAUTHORIZED", response.getProblem().getCode());
        assertEquals("auth", response.getProblem().getKind());
        assertEquals("corr-123", response.getProblem().getCorrelationId());
        assertEquals("check token", response.getProblem().getDetails().get("hint"));
    }

    @Test
    void ronProblemShouldBeImmutableAndDefensive() {
        Map<String, Object> details = new HashMap<>();
        details.put("foo", "bar");

        RonProblem problem = new RonProblem(
                "RATE_LIMITED",
                "Too many requests",
                "rate_limit",
                "corr-xyz",
                details
        );

        // Mutate original map — problem should not see this if it is defensive.
        details.put("foo", "baz");

        Map<String, Object> fromProblem = problem.getDetails();
        assertEquals("bar", fromProblem.get("foo"), "RonProblem should defensively copy details");

        // Ensure unmodifiable / safe to pass around
        assertThrows(UnsupportedOperationException.class, () -> {
            fromProblem.put("new", "value");
        }, "Details map should be unmodifiable");
    }

    @Test
    void appResponseStaticFactoryForEmptyErrorIsConvenient() {
        RonProblem problem = new RonProblem(
                "APP_ERROR",
                "Something bad happened",
                "app",
                null,
                Collections.emptyMap()
        );

        AppResponse<String> response = AppResponse.error(problem, 500);
        assertFalse(response.ok());
        assertEquals("APP_ERROR", response.getProblem().getCode());
        assertEquals(500, response.getStatus());
    }
}
