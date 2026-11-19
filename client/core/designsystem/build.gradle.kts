plugins {
    id("witness.kmp.library")
    id("witness.cmp")
}

compose.resources {
    packageOfResClass = "app.witness.core.designsystem.generated"
}

kotlin {
    sourceSets.getByName("androidHostTest").dependencies {
        implementation(libs.zxing.core)
    }
}

kotlin {
    sourceSets.wasmJsTest.dependencies {
        implementation(libs.compose.ui.test)
    }
}
