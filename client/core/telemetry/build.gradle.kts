plugins {
    id("witness.kmp.library")
}

kotlin {
    sourceSets {
        commonMain.dependencies { api(projects.core.domain) }
        androidMain.dependencies {
            implementation(project.dependencies.platform(libs.firebase.bom))
            implementation(libs.firebase.analytics)
            implementation(libs.firebase.crashlytics)
        }
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.kotlin.test.junit)
        }
    }
}
