pluginManagement {
    includeBuild("build-logic")
    repositories {
        google {
            content {
                includeGroupByRegex("androidx.*")
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("androidx.*")
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
            }
        }
        mavenCentral()
    }
}

plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")
rootProject.name = "witness"

include(":native:witness")
listOf("model", "designsystem", "ui", "domain", "verify", "icp", "telemetry", "data", "platform", "a2ui")
    .forEach { include(":core:$it") }
listOf("home", "capture", "post", "case", "share", "settings", "check", "proof")
    .forEach { include(":feature:$it") }
include(":androidApp", ":androidBenchmark", ":webApp")
