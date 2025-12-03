package dev.roncore.sdk.kotlin

import dev.roncore.sdk.AppResponse
import dev.roncore.sdk.RonClient
import dev.roncore.sdk.config.RonConfig
import dev.roncore.sdk.http.HttpClientAdapter
import dev.roncore.sdk.http.HttpRequestContext
import dev.roncore.sdk.http.HttpResponse
import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * RO:WHAT —
 *   Tests for the Kotlin coroutine extensions over RonClient.
 *
 * RO:WHY —
 *   Ensures the suspend functions delegate correctly to the underlying
 *   RonClient while remaining safe to call from coroutines.
 *
 * RO:INVARIANTS —
 *   - No real network calls are made; we use a fake HttpClientAdapter.
 *   - The coroutine wrapper that takes a KClass decodes the JSON
 *     "data" field into a simple Map, which does not require any
 *     Kotlin-specific Jackson modules.
 */
class RonCoroutineTest {

    /**
     * Minimal HttpClientAdapter that records the last request and returns
     * a synthetic JSON response envelope.
     */
    class RecordingHttpClientAdapter(
        private val body: String
    ) : HttpClientAdapter {

        @Volatile
        var lastRequest: HttpRequestContext? = null
            private set

        override fun execute(request: HttpRequestContext): HttpResponse {
            lastRequest = request
            return HttpResponse(
                200,
                body,
                emptyMap()
            )
        }
    }

    @Test
    fun `KClass-based coroutine get delegates to RonClient and decodes envelope into Map`() = runBlocking {
        // Arrange: fake adapter returns a valid AppResponse envelope whose "data"
        // is a simple object that can be mapped into Map<String, Object>.
        val jsonEnvelope = """
            {
              "data": { "value": "hello-from-test" },
              "problem": null
            }
        """.trimIndent()

        val adapter = RecordingHttpClientAdapter(jsonEnvelope)

        val config = RonConfig.builder()
            .baseUrl("https://node.example.com")
            .build()

        val client = RonClient.builder()
            .config(config)
            .httpClientAdapter(adapter)
            .build()

        // Act: use the Kotlin KClass-based extension:
        //   suspend fun <T : Any> RonClient.get(path: String, type: KClass<T>): AppResponse<T>
        //
        // We ask for a Map so Jackson can materialize the object without any
        // Kotlin-specific modules.
        @Suppress("UNCHECKED_CAST")
        val response: AppResponse<Map<String, Any?>> =
            client.get("/ping", Map::class) as AppResponse<Map<String, Any?>>

        // Assert: the response is OK and the data was decoded as expected.
        assertTrue(response.ok())
        val data = response.data
        assertNotNull(data)
        assertEquals("hello-from-test", data!!["value"])

        // Also ensure the adapter actually saw a request.
        val seen = adapter.lastRequest
        assertNotNull(seen)
        // Sanity-check that we normalized to /app/* on the gateway side.
        assertTrue(seen!!.url.startsWith("https://node.example.com/app/"))
    }
}
