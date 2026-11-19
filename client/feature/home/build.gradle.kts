plugins {
    id("witness.kmp.android")
    id("witness.cmp")
    alias(libs.plugins.kotlin.serialization)
}

kotlin {
    android {
        compilations.withType<com.android.build.api.dsl.KotlinMultiplatformAndroidHostTestCompilation>().configureEach {
            isIncludeAndroidResources = true
        }
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
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.kotlin.test.junit)
        }
    }
}

compose.resources {
    packageOfResClass = "app.witness.feature.home.generated"
}
