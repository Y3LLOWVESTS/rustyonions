plugins {
    `java-library`
    id("checkstyle")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
    withJavadocJar()
    withSourcesJar()
}

val libs = project.extensions.getByType<org.gradle.accessors.dm.LibrariesForLibs>()

dependencies {
    api(libs.jackson.annotations) // visible in public DTOs/envelopes if needed

    implementation(libs.okhttp)
    implementation(libs.jackson.core)
    implementation(libs.jackson.databind)
    implementation(libs.jackson.jsr310)

    testImplementation(libs.junit.jupiter)
}

tasks.withType<Checkstyle> {
    // You can wire config/checkstyle.xml later; using defaults for now.
    isIgnoreFailures = false
}
