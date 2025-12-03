package dev.roncore.sdk.kotlin

import dev.roncore.sdk.config.EnvConfigLoader
import dev.roncore.sdk.config.RonConfig
import java.time.Duration

/**
 * RO:WHAT —
 *   Kotlin DSL for building {@link RonConfig} on top of env defaults.
 *
 * RO:WHY —
 *   JVM apps can keep behavior consistent with other SDKs by
 *   starting from RON_SDK_* env vars, then layering Kotlin overrides.
 *
 * RO:INVARIANTS —
 *   - Env provides sane defaults; explicit DSL wins.
 *   - No secrets (tokens) are handled here; only transport config.
 */
@DslMarker
annotation class RonDslMarker

@RonDslMarker
class RonConfigDsl internal constructor() {

    /**
     * Base URL of the gateway, e.g. "https://127.0.0.1:5304".
     * If null, falls back to RON_SDK_GATEWAY_ADDR env var.
     */
    var baseUrl: String? = null

    /**
     * Allow plain HTTP. Only for dev/test, never for production.
     * If null, uses RON_SDK_INSECURE_HTTP semantics.
     */
    var insecureHttp: Boolean? = null

    var connectTimeoutMs: Long? = null
    var readTimeoutMs: Long? = null
    var writeTimeoutMs: Long? = null
    var overallTimeoutMs: Long? = null

    var maxRetries: Int? = null
    var maxResponseBytes: Long? = null
}

/**
 * Builds a {@link RonConfig} by:
 *
 *   1. Starting from RON_SDK_* env vars (when present).
 *   2. Applying any overrides from the [block].
 */
fun ronConfig(block: RonConfigDsl.() -> Unit = {}): RonConfig {
    val dsl = RonConfigDsl().apply(block)
    val builder = EnvConfigLoader.fromEnv()

    dsl.baseUrl?.let { builder.baseUrl(it) }

    dsl.connectTimeoutMs?.let { builder.connectTimeout(Duration.ofMillis(it)) }
    dsl.readTimeoutMs?.let { builder.readTimeout(Duration.ofMillis(it)) }
    dsl.writeTimeoutMs?.let { builder.writeTimeout(Duration.ofMillis(it)) }
    dsl.overallTimeoutMs?.let { builder.overallTimeout(Duration.ofMillis(it)) }

    dsl.insecureHttp?.let { builder.allowInsecureHttp(it) }
    dsl.maxRetries?.let { builder.maxRetries(it) }
    dsl.maxResponseBytes?.let { builder.maxResponseBytes(it) }

    return builder.build()
}
