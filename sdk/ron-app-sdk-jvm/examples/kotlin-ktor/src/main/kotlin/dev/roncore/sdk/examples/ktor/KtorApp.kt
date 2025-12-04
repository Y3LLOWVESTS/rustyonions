package dev.roncore.sdk.examples.ktor

import dev.roncore.sdk.RonClient
import dev.roncore.sdk.RonException
import io.ktor.serialization.jackson.jackson
import io.ktor.server.application.Application
import io.ktor.server.application.install
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.plugins.callloging.CallLogging
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.routing.routing
import org.slf4j.LoggerFactory

fun main() {
    val port = System.getenv("RON_EXAMPLE_KTOR_PORT")?.toIntOrNull() ?: 8080

    embeddedServer(
        Netty,
        port = port,
        host = "0.0.0.0"
    ) {
        ronExampleModule()
    }.start(wait = true)
}

fun Application.ronExampleModule() {
    val logger = LoggerFactory.getLogger("ron-example-ktor")

    install(CallLogging)

    install(ContentNegotiation) {
        jackson {
            findAndRegisterModules()
        }
    }

    val ronClient: RonClient? = try {
        RonClient.builder()
            .fromEnv() // Uses RON_SDK_GATEWAY_ADDR, RON_SDK_INSECURE_HTTP, etc.
            .build()
    } catch (ex: RonException) {
        logger.error("Failed to create RonClient from env; /ron/* routes will return 500", ex)
        null
    }

    routing {
        healthRoutes()
        ronRoutes(ronClient)
    }

    logger.info("Ktor example started on /healthz and /ron/ping")
}
