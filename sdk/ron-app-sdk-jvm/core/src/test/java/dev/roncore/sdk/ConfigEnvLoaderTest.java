package dev.roncore.sdk;

import dev.roncore.sdk.config.EnvConfigLoader;
import dev.roncore.sdk.config.RonConfig;
import org.junit.jupiter.api.Test;

import java.util.HashMap;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Env config loader tests.
 *
 * RO:WHAT  — Verifies that EnvConfigLoader maps canonical env vars into RonConfig.
 * RO:WHY   — Mirrors TS SDK behavior: env vars provide sensible defaults but
 *            code options always win; ensures shared var names behave as expected.
 * RO:INTERACTS —
 *   - EnvConfigLoader
 *   - RonConfig.Builder
 * RO:INVARIANTS —
 *   - RON_SDK_GATEWAY_ADDR populates baseUrl when not set explicitly.
 *   - Timeout envs map to timeout fields.
 *   - Explicit builder values win over env defaults.
 */
public class ConfigEnvLoaderTest {

    @Test
    void envVarsPopulateConfigDefaults() {
        Map<String, String> env = new HashMap<>();
        env.put("RON_SDK_GATEWAY_ADDR", "https://node.env.example.com");
        env.put("RON_SDK_OVERALL_TIMEOUT_MS", "15000");
        env.put("RON_SDK_CONNECT_TIMEOUT_MS", "2000");
        env.put("RON_SDK_READ_TIMEOUT_MS", "4000");
        env.put("RON_SDK_WRITE_TIMEOUT_MS", "5000");

        RonConfig.Builder builder = RonConfig.builder();
        EnvConfigLoader.applyEnv(builder, env);

        RonConfig cfg = builder.build();

        assertEquals("https://node.env.example.com", cfg.getBaseUrl());
        assertEquals(15_000L, cfg.getOverallTimeoutMs());
        assertEquals(2_000L, cfg.getConnectTimeoutMs());
        assertEquals(4_000L, cfg.getReadTimeoutMs());
        assertEquals(5_000L, cfg.getWriteTimeoutMs());
    }

    @Test
    void explicitConfigOverridesEnvDefaults() {
        Map<String, String> env = new HashMap<>();
        env.put("RON_SDK_GATEWAY_ADDR", "https://node.env.example.com");

        RonConfig.Builder builder = RonConfig.builder()
                .baseUrl("https://node.code.example.com");

        EnvConfigLoader.applyEnv(builder, env);

        RonConfig cfg = builder.build();

        // Builder-provided value should win over env default.
        assertEquals("https://node.code.example.com", cfg.getBaseUrl());
    }

    @Test
    void missingEnvAndConfigShouldStillFailOnBuild() {
        RonConfig.Builder builder = RonConfig.builder();
        // No env, no baseUrl
        EnvConfigLoader.applyEnv(builder, new HashMap<>());

        RuntimeException ex = assertThrows(
                RuntimeException.class,
                builder::build,
                "Building config without baseUrl should fail even after env load"
        );

        String msg = ex.getMessage() == null ? "" : ex.getMessage().toLowerCase();
        assertTrue(
                msg.contains("baseurl") || msg.contains("base url"),
                "Error message should hint at missing baseUrl; got: " + ex.getMessage()
        );
    }
}
