plugins {
    id("witness.kmp.library")
    id("witness.cmp")
    alias(libs.plugins.kotlin.serialization)
}

kotlin {
    sourceSets.matching { it.name.startsWith("web") || it.name.startsWith("js") || it.name.startsWith("wasm") }.configureEach {
        languageSettings.optIn("kotlin.js.ExperimentalWasmJsInterop")
    }
    sourceSets {
        commonMain.dependencies {
            implementation(projects.core.domain)
            implementation(projects.core.ui)
            implementation(libs.jetbrains.navigation3.ui)
            implementation(libs.kotlinx.serialization.json)
        }
        androidMain.dependencies {
            implementation(libs.androidx.activity.compose)
        }
        webMain.dependencies {
            implementation(libs.kotlinx.browser)
        }
    }
}
