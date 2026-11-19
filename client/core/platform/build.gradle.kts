plugins {
    id("witness.kmp.android")
    alias(libs.plugins.ksp)
}

kotlin {
    android {
        optimization {
            consumerKeepRules.publish = true
            consumerKeepRules.file("consumer-rules.pro")
        }
    }
    sourceSets {
        commonMain.dependencies { api(projects.core.domain) }
        androidMain.dependencies {
            implementation(libs.androidx.camera.core)
            implementation(libs.androidx.camera.camera2)
            implementation(libs.androidx.camera.lifecycle)
            implementation(libs.androidx.camera.compose)
            implementation(libs.mlkit.face.detection)
            implementation(libs.mlkit.text.recognition)
            implementation(libs.mlkit.genai.prompt)
        }
        getByName("androidHostTest").dependencies {
            implementation(libs.robolectric)
            implementation(libs.androidx.test.core)
            implementation(libs.kotlin.test.junit)
        }
    }
}

dependencies {
    add("kspAndroid", libs.mlkit.genai.schema.compiler)
}
