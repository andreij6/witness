import com.android.build.api.dsl.ApplicationExtension
import org.gradle.api.JavaVersion
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.configure
import org.gradle.kotlin.dsl.dependencies
import org.gradle.kotlin.dsl.withType
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinJvmCompile

class AndroidAppConventionPlugin : Plugin<Project> {
    override fun apply(target: Project) = with(target) {
        pluginManager.apply("com.android.application")
        pluginManager.apply("org.jetbrains.kotlin.plugin.compose")
        if (file("google-services.json").exists()) {
            pluginManager.apply("com.google.gms.google-services")
            pluginManager.apply("com.google.firebase.crashlytics")
        }
        val catalog = libs
        extensions.configure<ApplicationExtension> {
            compileSdk { version = release(catalog.sdk("compileSdk")) { minorApiLevel = catalog.sdk("compileSdkMinor") } }
            defaultConfig {
                minSdk = catalog.sdk("minSdk")
                targetSdk = catalog.sdk("targetSdk")
                testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
            }
            compileOptions {
                sourceCompatibility = JavaVersion.VERSION_17
                targetCompatibility = JavaVersion.VERSION_17
            }
            buildFeatures {
                compose = true
                buildConfig = false
                aidl = false
                shaders = false
                resValues = false
            }
            packaging {
                resources.excludes += setOf(
                    "/META-INF/{AL2.0,LGPL2.1}",
                    "/META-INF/*.version",
                    "DebugProbesKt.bin",
                    "kotlin-tooling-metadata.json",
                )
            }
        }
        tasks.withType<KotlinJvmCompile>().configureEach {
            compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
        }
        dependencies {
            add("implementation", catalog.lib("androidx-activity-compose"))
            add("debugImplementation", catalog.lib("compose-ui-tooling"))
            add("testImplementation", catalog.lib("kotlin-test-junit"))
            add("testImplementation", catalog.lib("kotlinx-coroutines-test"))
        }
        Unit
    }
}
