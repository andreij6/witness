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
            api(libs.jetbrains.navigation3.ui)
        }
        androidMain.dependencies {
            implementation(libs.revenuecat.purchases.core)
        }
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.androidx.activity.compose)
            implementation(libs.kotlin.test.junit)
        }
    }
}
