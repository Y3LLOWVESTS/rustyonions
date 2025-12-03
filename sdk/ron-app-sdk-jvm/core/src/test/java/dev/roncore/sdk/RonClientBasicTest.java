package dev.roncore.sdk;

import dev.roncore.sdk.config.RonConfig;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Basic construction / config sanity tests for RonClient.
 *
 * RO:WHAT  — Smoke tests for the public RonClient + RonConfig builder.
 * RO:WHY   — Catch obvious misconfiguration (missing baseUrl, invalid URL)
 *            and ensure the “happy path” builder round-trip works.
 * RO:INTERACTS —
 *   - RonClient (core entry point)
 *   - RonConfig (config DTO / builder)
 * RO:INVARIANTS —
 *   - Missing baseUrl should fail fast (CONFIG_MISSING_BASE_URL).
 *   - Well-formed HTTPS baseUrl should build without error.
 */
public class RonClientBasicTest {

    @Test
    void buildWithoutBaseUrlShouldFailFast() {
        RuntimeException ex = assertThrows(
                RuntimeException.class,
                () -> RonClient.builder().build(),
                "Building a client without baseUrl should fail fast"
        );

        // We do not hard-code the exact exception type yet, only that it is
        // a runtime failure and the message hints about baseUrl / config.
        String msg = ex.getMessage() == null ? "" : ex.getMessage().toLowerCase();
        assertTrue(
                msg.contains("baseurl") || msg.contains("base url") || msg.contains("config"),
                "Error message should mention baseUrl or config; got: " + ex.getMessage()
        );
    }

    @Test
    void buildWithHttpsBaseUrlShouldSucceed() {
        RonClient client = RonClient.builder()
                .baseUrl("https://node.example.com")
                .build();

        assertNotNull(client, "RonClient should be created when baseUrl is provided");
    }

    @Test
    void configBuilderRoundTripKeepsValues() {
        RonConfig config = RonConfig.builder()
                .baseUrl("https://node.example.com")
                .overallTimeoutMs(10_000L)
                .connectTimeoutMs(2_000L)
                .readTimeoutMs(5_000L)
                .writeTimeoutMs(5_000L)
                .allowInsecureHttp(false)
                .build();

        assertEquals("https://node.example.com", config.getBaseUrl());
        assertEquals(10_000L, config.getOverallTimeoutMs());
        assertEquals(2_000L, config.getConnectTimeoutMs());
        assertEquals(5_000L, config.getReadTimeoutMs());
        assertEquals(5_000L, config.getWriteTimeoutMs());
        assertFalse(config.isAllowInsecureHttp(), "HTTPS should be the default / recommended path");
    }
}
