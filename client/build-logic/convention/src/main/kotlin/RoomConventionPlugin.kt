import androidx.room3.gradle.RoomExtension
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.configure

class RoomConventionPlugin : Plugin<Project> {
    override fun apply(target: Project) = with(target) {
        pluginManager.apply("com.google.devtools.ksp")
        pluginManager.apply("androidx.room3")
        val catalog = libs
        extensions.configure<RoomExtension> { schemaDirectory(layout.projectDirectory.dir("schemas")) }
        kmp.apply {
            sourceSets.commonMain.dependencies {
                implementation(catalog.lib("androidx-room3-runtime"))
                implementation(catalog.lib("androidx-sqlite-bundled"))
            }
        }
        dependencies.add("kspAndroid", catalog.lib("androidx-room3-compiler"))
        Unit
    }
}
