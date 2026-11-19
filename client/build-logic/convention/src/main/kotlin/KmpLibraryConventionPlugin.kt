import org.gradle.api.Plugin
import org.gradle.api.Project
import org.jetbrains.kotlin.gradle.ExperimentalWasmDsl

class KmpLibraryConventionPlugin : Plugin<Project> {
    @OptIn(ExperimentalWasmDsl::class)
    override fun apply(target: Project) = with(target) {
        configureKmpAndroid()
        kmp.apply {
            js { browser { testTask { useKarma { useChromeHeadless() } } } }
            wasmJs { browser { testTask { useKarma { useChromeHeadless() } } } }
        }
        Unit
    }
}
