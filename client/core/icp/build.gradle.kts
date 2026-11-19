plugins {
    id("witness.kmp.library")
}

kotlin {
    js { useEsModules() }
    sourceSets.matching { it.name.startsWith("web") || it.name.startsWith("js") || it.name.startsWith("wasm") }.configureEach {
        languageSettings.optIn("kotlin.js.ExperimentalWasmJsInterop")
    }
    sourceSets {
        commonMain.dependencies {
            api(projects.core.domain)
            implementation(libs.kotlinx.serialization.json)
        }
        webMain.dependencies {
            implementation(libs.kotlinx.browser)
            implementation(npm("witness-bridges", rootProject.file("webApp/bridges")))
        }
    }
}
