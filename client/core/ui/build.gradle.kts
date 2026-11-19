plugins {
    id("witness.kmp.library")
    id("witness.cmp")
}

kotlin {
    sourceSets.commonMain.dependencies {
        api(projects.core.model)
        api(projects.core.designsystem)
    }
    sourceSets.wasmJsTest.dependencies {
        implementation(libs.compose.ui.test)
    }
}
