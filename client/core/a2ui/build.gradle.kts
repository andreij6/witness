plugins {
    id("witness.kmp.android")
    id("witness.cmp")
    alias(libs.plugins.kotlin.serialization)
}

kotlin {
    sourceSets {
        commonMain.dependencies {
            api(projects.core.domain)
            implementation(projects.core.ui)
            api(libs.kotlinx.serialization.json)
        }
        androidMain.dependencies {
            api(libs.androidx.a2ui.model)
            implementation(libs.androidx.a2ui.engine)
            api(libs.androidx.a2ui.compose.runtime)
            api(libs.androidx.a2ui.compose.ui)
        }
    }
}

compose.resources {
    packageOfResClass = "app.witness.core.a2ui.resources"
}
