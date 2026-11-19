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
    sourceSets.getByName("androidHostTest").dependencies {
        implementation(libs.robolectric)
        implementation(libs.androidx.test.core)
        implementation(libs.androidx.activity.compose)
        implementation(libs.kotlin.test.junit)
    }
    sourceSets.commonMain.dependencies {
        implementation(projects.core.domain)
        implementation(projects.core.ui)
        implementation(libs.jetbrains.navigation3.ui)
    }
    sourceSets.androidMain.dependencies {
        implementation(libs.androidx.camera.core)
        implementation(libs.androidx.camera.lifecycle)
        implementation(libs.androidx.camera.view)
    }
}
