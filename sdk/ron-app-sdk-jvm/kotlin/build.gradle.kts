plugins {
    id("org.jetbrains.kotlin.jvm")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
    implementation(project(":core"))

    implementation(libs.kotlin.coroutines.core)

    testImplementation(libs.junit.jupiter)
}

tasks.test {
    useJUnitPlatform()
}
