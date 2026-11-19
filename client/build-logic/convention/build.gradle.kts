plugins {
    `kotlin-dsl`
}

group = "app.witness.buildlogic"

kotlin {
    jvmToolchain(21)
}

dependencies {
    compileOnly(libs.android.gradlePlugin)
    compileOnly(libs.kotlin.gradlePlugin)
    compileOnly(libs.compose.compiler.gradlePlugin)
    compileOnly(libs.compose.multiplatform.gradlePlugin)
    compileOnly(libs.ksp.gradlePlugin)
    compileOnly(libs.metro.gradlePlugin)
    compileOnly(libs.room3.gradlePlugin)
    testImplementation(libs.kotlin.test.junit)
}

gradlePlugin {
    plugins {
        register("kmpLibrary") { id = "witness.kmp.library"; implementationClass = "KmpLibraryConventionPlugin" }
        register("kmpAndroid") { id = "witness.kmp.android"; implementationClass = "KmpAndroidConventionPlugin" }
        register("cmp") { id = "witness.cmp"; implementationClass = "CmpConventionPlugin" }
        register("androidApp") { id = "witness.android.app"; implementationClass = "AndroidAppConventionPlugin" }
        register("webApp") { id = "witness.web.app"; implementationClass = "WebAppConventionPlugin" }
        register("metro") { id = "witness.metro"; implementationClass = "MetroConventionPlugin" }
        register("room") { id = "witness.room"; implementationClass = "RoomConventionPlugin" }
    }
}
