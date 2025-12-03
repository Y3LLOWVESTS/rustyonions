plugins {
    id("org.jetbrains.kotlin.jvm")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
    // Facets are intentionally lightweight: no core/HTTP deps.
    testImplementation(libs.junit.jupiter)
}

tasks.test {
    useJUnitPlatform()
}
