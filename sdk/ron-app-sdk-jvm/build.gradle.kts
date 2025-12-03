import org.gradle.api.tasks.testing.logging.TestLogEvent

plugins {
    // Only third-party plugins belong here with versions.
    id("org.jetbrains.kotlin.jvm") version "1.9.25" apply false
    id("org.jlleitschuh.gradle.ktlint") version "12.1.1" apply false
    id("io.gitlab.arturbosch.detekt") version "1.23.6" apply false
    // NOTE:
    // Core Gradle plugins like `java-library` and `checkstyle` are applied
    // directly in subprojects (e.g. core/build.gradle.kts) and MUST NOT be
    // declared here with `apply false`.
}

allprojects {
    group = "dev.roncore"
    version = "0.1.0-SNAPSHOT"

    repositories {
        mavenCentral()
    }
}

subprojects {
    tasks.withType<Test>().configureEach {
        useJUnitPlatform()

        testLogging {
            events = setOf(
                TestLogEvent.PASSED,
                TestLogEvent.SKIPPED,
                TestLogEvent.FAILED
            )
        }
    }
}
