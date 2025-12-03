package dev.roncore.sdk.kotlin

import dev.roncore.sdk.AppResponse
import dev.roncore.sdk.RonClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlin.reflect.KClass

/**
 * RO:WHAT —
 *   Extension functions adding suspend + reified sugar on top of RonClient.
 *
 * RO:WHY —
 *   Keeps base Java client small and predictable while giving
 *   Kotlin callers a pleasant, type-safe API.
 */

@Suppress("unused")
suspend inline fun <reified T : Any> RonClient.get(path: String): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@get.get(path, T::class.java)
    }

@Suppress("unused")
suspend inline fun <reified T : Any> RonClient.post(path: String, body: Any?): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@post.post(path, body, T::class.java)
    }

@Suppress("unused")
suspend inline fun <reified T : Any> RonClient.put(path: String, body: Any?): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@put.put(path, body, T::class.java)
    }

@Suppress("unused")
suspend inline fun <reified T : Any> RonClient.patch(path: String, body: Any?): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@patch.patch(path, body, T::class.java)
    }

@Suppress("unused")
suspend inline fun <reified T : Any> RonClient.delete(path: String): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@delete.delete(path, T::class.java)
    }

/**
 * Variant that accepts a KClass (handy for reflection-heavy call sites).
 */
@Suppress("unused")
suspend fun <T : Any> RonClient.get(path: String, type: KClass<T>): AppResponse<T> =
    withContext(Dispatchers.IO) {
        this@get.get(path, type.java)
    }
