plugins {
    id("witness.kmp.library")
    id("witness.cmp")
    alias(libs.plugins.kotlin.serialization)
}

kotlin {
    sourceSets.commonMain.dependencies {
        implementation(projects.core.domain)
        implementation(projects.core.ui)
        implementation(libs.jetbrains.navigation3.ui)
        implementation(libs.kotlinx.serialization.json)
    }
}
