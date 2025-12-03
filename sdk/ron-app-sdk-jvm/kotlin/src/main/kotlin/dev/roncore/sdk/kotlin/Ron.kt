package dev.roncore.sdk.kotlin

import dev.roncore.sdk.AppResponse
import dev.roncore.sdk.RonClient
import dev.roncore.sdk.config.RonConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * RO:WHAT —
 *   Kotlin-first facade over {@link RonClient}.
 *
 * RO:WHY —
 *   Gives Kotlin apps a nicer entrypoint with coroutines and
 *   a config DSL, without hiding the underlying Java client.
 *
 * RO:INVARIANTS —
 *   - One Ron instance wraps one RonClient.
 *   - Safe to share across coroutines; underlying RonClient is thread-safe.
 */
class Ron private constructor(
    private val client: RonClient
) : AutoCloseable {

    companion object {

        /**
         * Build a Ron instance from a pre-built {@link RonConfig}.
         */
        @JvmStatic
        fun fromConfig(config: RonConfig): Ron {
            val client = RonClient.builder()
                .config(config)
                .build()
            return Ron(client)
        }

        /**
         * Build a Ron instance from env + Kotlin DSL overrides.
         *
         * Example:
         *   val ron = Ron.fromEnv {
         *       baseUrl = "https://127.0.0.1:5304"
         *       insecureHttp = true
         *   }
         */
        @JvmStatic
        fun fromEnv(block: RonConfigDsl.() -> Unit = {}): Ron {
            val config = ronConfig(block)
            val client = RonClient.builder()
                .config(config)
                .build()
            return Ron(client)
        }
    }

    /**
     * Coroutine-friendly GET wrapper returning an AppResponse.
     */
    suspend fun <T> get(path: String, clazz: Class<T>): AppResponse<T> =
        withContext(Dispatchers.IO) {
            client.get(path, clazz)
        }

    suspend fun getString(path: String): AppResponse<String> =
        get(path, String::class.java)

    override fun close() {
        client.close()
    }
}
