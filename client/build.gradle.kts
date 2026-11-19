plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.kotlin.multiplatform.library) apply false
    alias(libs.plugins.android.test) apply false
    alias(libs.plugins.kotlin.multiplatform) apply false
    alias(libs.plugins.kotlin.serialization) apply false
    alias(libs.plugins.compose.compiler) apply false
    alias(libs.plugins.compose.multiplatform) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.metro) apply false
    alias(libs.plugins.room3) apply false
    alias(libs.plugins.google.services) apply false
    alias(libs.plugins.firebase.crashlytics) apply false
    alias(libs.plugins.baselineprofile) apply false
}

val bridgeSources = fileTree("webApp/bridges") { include("*.mjs", "package.json", "wasm/**") }
val installedBridges = listOf("js", "wasm").flatMap { listOf(layout.buildDirectory.file("$it/node_modules/witness-bridges").get().asFile, layout.buildDirectory.file("$it/node_modules/.yarn-integrity").get().asFile) }

tasks.named { it == "kotlinNpmInstall" || it == "kotlinWasmNpmInstall" }.configureEach {
    inputs.files(bridgeSources).withPropertyName("witnessBridges").withPathSensitivity(PathSensitivity.RELATIVE)
    val stale = installedBridges
    doFirst { stale.forEach(File::deleteRecursively) }
}
