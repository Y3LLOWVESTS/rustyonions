package dev.roncore.sdk.kotlin

import dev.roncore.sdk.config.RonConfig
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * RO:WHAT —
 *   Tests for the Kotlin ronConfig DSL.
 *
 * RO:WHY —
 *   Ensures that the DSL correctly applies explicit overrides on top of
 *   the EnvConfigLoader defaults and produces a consistent RonConfig.
 *
 * RO:INVARIANTS —
 *   - DSL must be usable without any RON_SDK_* env vars set.
 *   - Explicit values in the DSL win over env/defaults.
 *   - All timeouts and limits round-trip as expected.
 */
class RonDslTest {

    @Test
    fun `ronConfig applies explicit overrides`() {
        val cfg: RonConfig = ronConfig {
            baseUrl = "https://node.example.com"
            insecureHttp = true

            connectTimeoutMs = 1_234L
            readTimeoutMs = 2_345L
            writeTimeoutMs = 3_456L
            overallTimeoutMs = 4_567L

            maxRetries = 5
            maxResponseBytes = 1024L * 1024L // 1 MiB
        }

        // Base URL + HTTPS invariants
        assertEquals("https://node.example.com", cfg.baseUrl)
        assertTrue(cfg.baseUri.toString().startsWith("https://"))

        // Timeouts (millis)
        assertEquals(1_234L, cfg.connectTimeout.toMillis())
        assertEquals(2_345L, cfg.readTimeout.toMillis())
        assertEquals(3_456L, cfg.writeTimeout.toMillis())
        assertEquals(4_567L, cfg.overallTimeout.toMillis())

        // Insecure HTTP flag + limits
        assertTrue(cfg.isAllowInsecureHttp())
        assertEquals(5, cfg.maxRetries)
        assertEquals(1024L * 1024L, cfg.maxResponseBytes)
    }
}
