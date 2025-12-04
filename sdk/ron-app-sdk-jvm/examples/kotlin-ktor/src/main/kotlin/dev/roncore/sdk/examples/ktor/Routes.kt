package dev.roncore.sdk.examples.ktor

import dev.roncore.sdk.AppResponse
import dev.roncore.sdk.RonClient
import dev.roncore.sdk.RonException
import dev.roncore.sdk.RonProblem
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.call
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

fun Route.healthRoutes() {
    get("/healthz") {
        call.respondText("ok")
    }
}

fun Route.ronRoutes(ronClient: RonClient?) {
    get("/ron/ping") {
        if (ronClient == null) {
            call.respond(
                HttpStatusCode.InternalServerError,
                mapOf(
                    "ok" to false,
                    "error" to "RonClient not configured; check RON_SDK_GATEWAY_ADDR / env."
                )
            )
            return@get
        }

        val response: AppResponse<*> = try {
            withContext(Dispatchers.IO) {
                @Suppress("UNCHECKED_CAST")
                ronClient.get("/ping", Map::class.java as Class<Map<String, Any>>)
            }
        } catch (ex: RonException) {
            call.respond(
                HttpStatusCode.BadGateway,
                mapOf(
                    "ok" to false,
                    "kind" to ex.kind,
                    "code" to ex.code,
                    "message" to (ex.message ?: "RON-CORE call failed"),
                    "retryable" to ex.isRetryable,
                    "details" to ex.details
                )
            )
            return@get
        }

        if (response.ok()) {
            @Suppress("UNCHECKED_CAST")
            val data = (response.data as? Map<String, Any>) ?: emptyMap<String, Any>()

            call.respond(
                HttpStatusCode.OK,
                mapOf(
                    "ok" to true,
                    "status" to response.status,
                    "data" to data
                )
            )
        } else {
            val problem: RonProblem? = response.problem

            call.respond(
                HttpStatusCode.fromValue(response.status),
                mapOf(
                    "ok" to false,
                    "status" to response.status,
                    "problem" to problem
                )
            )
        }
    }
}
