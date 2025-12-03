plugins {
    java
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
    // Interop tests exercise the real RonClient against a live gateway.
    testImplementation(project(":core"))
    testImplementation(libs.junit.jupiter)
}

tasks.test {
    // Root build already configures JUnit Platform, but we keep this explicit.
    useJUnitPlatform()
}
