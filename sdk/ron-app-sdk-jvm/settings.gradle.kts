pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
    plugins {
        id("com.android.application") version "8.2.0"
    }
}

rootProject.name = "ron-app-sdk-jvm"

include("core")
include("kotlin")
include("facets")

include("examples:java-cli")
include("examples:spring-boot")
include("examples:kotlin-ktor")
include("examples:android-sample")

include("interop-tests")
