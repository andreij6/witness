plugins {
    id("witness.web.app")
}

kotlin {
    sourceSets.matching { it.name.startsWith("web") || it.name.startsWith("js") || it.name.startsWith("wasm") }.configureEach {
        languageSettings.optIn("kotlin.js.ExperimentalWasmJsInterop")
    }
    sourceSets.commonMain.dependencies {
        implementation(projects.core.domain)
        implementation(projects.core.icp)
        implementation(projects.core.verify)
        implementation(projects.core.designsystem)
        implementation(projects.feature.check)
        implementation(projects.feature.proof)
        implementation(libs.jetbrains.navigation3.ui)
        implementation(libs.kotlinx.serialization.json)
    }
    sourceSets.commonTest.dependencies {
        implementation(libs.kotlinx.coroutines.test)
    }
    sourceSets.webMain.dependencies {
        implementation(libs.kotlinx.browser)
    }
}

val bridgeSources = fileTree(rootProject.file("webApp/bridges")) { include("*.mjs", "package.json", "wasm/**") }
tasks.withType<org.jetbrains.kotlin.gradle.targets.js.webpack.KotlinWebpack>().configureEach {
    inputs.files(bridgeSources).withPropertyName("witnessBridges").withPathSensitivity(PathSensitivity.RELATIVE)
}
