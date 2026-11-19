plugins {
    id("witness.kmp.android")
    id("witness.room")
}

kotlin {
    sourceSets {
        commonMain.dependencies { api(projects.core.domain) }
        androidMain.dependencies {
            implementation(libs.androidx.datastore.preferences)
            implementation(libs.androidx.work.runtime)
        }
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.kotlin.test.junit)
        }
    }
}
