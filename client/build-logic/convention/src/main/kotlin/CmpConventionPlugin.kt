import com.android.build.api.dsl.KotlinMultiplatformAndroidLibraryTarget
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.plugins.ExtensionAware

class CmpConventionPlugin : Plugin<Project> {
    override fun apply(target: Project) = with(target) {
        pluginManager.apply("org.jetbrains.compose")
        pluginManager.apply("org.jetbrains.kotlin.plugin.compose")
        val catalog = libs
        kmp.apply {
            (this as ExtensionAware).extensions.findByType(KotlinMultiplatformAndroidLibraryTarget::class.java)
                ?.androidResources?.enable = true
            sourceSets.commonMain.dependencies {
                composeLibraries.forEach { implementation(catalog.lib(it)) }
            }
        }
        tasks.matching { it.name.startsWith("checkComposeUiTestConfigurationFor") }.configureEach { enabled = false }
    }

    private val composeLibraries = listOf(
        "compose-runtime",
        "compose-foundation",
        "compose-ui",
        "compose-material3",
        "compose-components-resources",
        "compose-ui-tooling-preview",
        "jetbrains-lifecycle-runtime-compose",
        "jetbrains-lifecycle-viewmodel-compose",
        "kotlinx-collections-immutable",
    )
}
