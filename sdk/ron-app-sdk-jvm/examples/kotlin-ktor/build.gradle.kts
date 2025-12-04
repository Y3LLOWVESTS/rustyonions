import org.gradle.api.tasks.testing.logging.TestLogEvent

plugins {
    id("org.jetbrains.kotlin.jvm")
    application
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
    implementation(project(":core"))

    // Ktor server (Netty engine + JSON via Jackson)
    implementation("io.ktor:ktor-server-core-jvm:2.3.13")
    implementation("io.ktor:ktor-server-netty-jvm:2.3.13")
    implementation("io.ktor:ktor-server-content-negotiation-jvm:2.3.13")
    implementation("io.ktor:ktor-serialization-jackson-jvm:2.3.13")
    implementation("io.ktor:ktor-server-call-logging-jvm:2.3.13")

    // Reuse SDK’s Jackson + coroutines from the version catalog
    implementation(libs.jackson.core)
    implementation(libs.jackson.databind)
    implementation(libs.jackson.annotations)
    implementation(libs.kotlin.coroutines.core)

    testImplementation(libs.junit.jupiter)
}

application {
    // Top-level main() in KtorApp.kt
    mainClass.set("dev.roncore.sdk.examples.ktor.KtorAppKt")
}

tasks.test {
    useJUnitPlatform()

    testLogging {
        events = setOf(
            TestLogEvent.PASSED,
            TestLogEvent.SKIPPED,
            TestLogEvent.FAILED
        )
    }
}
