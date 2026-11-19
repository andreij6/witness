import org.gradle.api.Plugin
import org.gradle.api.Project
import org.jetbrains.kotlin.gradle.ExperimentalWasmDsl

class WebAppConventionPlugin : Plugin<Project> {
    @OptIn(ExperimentalWasmDsl::class)
    override fun apply(target: Project) = with(target) {
        pluginManager.apply("org.jetbrains.kotlin.multiplatform")
        val catalog = libs
        kmp.apply {
            js {
                browser {
                    commonWebpackConfig { outputFileName = BUNDLE }
                    testTask { useKarma { useChromeHeadless() } }
                }
                binaries.executable()
            }
            wasmJs {
                browser {
                    commonWebpackConfig { outputFileName = BUNDLE }
                    testTask { useKarma { useChromeHeadless() } }
                }
                binaries.executable()
            }
            sourceSets.commonTest.dependencies {
                implementation(catalog.lib("kotlin-test"))
            }
        }
        pluginManager.apply(CmpConventionPlugin::class.java)
    }

    private companion object {
        const val BUNDLE = "witness.js"
    }
}
