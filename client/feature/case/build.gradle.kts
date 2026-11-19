import com.android.build.api.dsl.KotlinMultiplatformAndroidHostTestCompilation
import com.android.build.api.dsl.KotlinMultiplatformAndroidLibraryTarget

plugins {
    id("witness.kmp.android")
    id("witness.cmp")
    alias(libs.plugins.kotlin.serialization)
}

kotlin {
    targets.withType<KotlinMultiplatformAndroidLibraryTarget>().configureEach {
        compilations.withType<KotlinMultiplatformAndroidHostTestCompilation>().configureEach { isIncludeAndroidResources = true }
    }
    sourceSets {
        commonMain.dependencies {
            implementation(projects.core.domain)
            implementation(projects.core.ui)
            implementation(libs.jetbrains.navigation3.ui)
            implementation(libs.kotlinx.serialization.json)
        }
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.kotlin.test.junit)
            implementation(libs.androidx.activity.compose)
        }
    }
}
