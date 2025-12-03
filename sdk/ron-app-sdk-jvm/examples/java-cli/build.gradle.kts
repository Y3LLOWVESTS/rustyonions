import org.gradle.api.tasks.testing.logging.TestLogEvent

plugins {
    `java`
    application
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
    implementation(project(":core"))

    testImplementation(libs.junit.jupiter)
}

tasks.test {
    useJUnitPlatform()

    testLogging {
        events = setOf(TestLogEvent.PASSED, TestLogEvent.SKIPPED, TestLogEvent.FAILED)
    }
}

application {
    // Matches the package + class below.
    mainClass.set("dev.roncore.sdk.examples.java.HelloCli")
}
