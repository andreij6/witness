import com.android.build.api.dsl.KotlinMultiplatformAndroidLibraryTarget
import org.gradle.api.Project
import org.gradle.api.artifacts.MinimalExternalModuleDependency
import org.gradle.api.artifacts.ProjectDependency
import org.gradle.api.artifacts.VersionCatalog
import org.gradle.api.artifacts.VersionCatalogsExtension
import org.gradle.api.plugins.ExtensionAware
import org.gradle.api.provider.Provider
import org.gradle.kotlin.dsl.configure
import org.gradle.kotlin.dsl.getByType
import org.gradle.kotlin.dsl.withType
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.dsl.KotlinMultiplatformExtension

internal val Project.libs: VersionCatalog
    get() = extensions.getByType<VersionCatalogsExtension>().named("libs")

internal fun VersionCatalog.lib(alias: String): Provider<MinimalExternalModuleDependency> = findLibrary(alias).get()

internal fun VersionCatalog.sdk(name: String): Int = findVersion("android-$name").get().requiredVersion.toInt()

internal val Project.kmp: KotlinMultiplatformExtension
    get() = extensions.getByType<KotlinMultiplatformExtension>()

internal fun KotlinMultiplatformExtension.androidLibrary(block: KotlinMultiplatformAndroidLibraryTarget.() -> Unit) =
    (this as ExtensionAware).extensions.configure<KotlinMultiplatformAndroidLibraryTarget>(block)

private val javaReservedSegments = mapOf("case" to "cases", "native" to "nativelib")

internal fun witnessNamespace(modulePath: String): String =
    modulePath.removePrefix(":").split(':')
        .joinToString(separator = ".", prefix = "app.witness.") { javaReservedSegments[it] ?: it }

private val featureAllowedDependencies = setOf(":core:model", ":core:designsystem", ":core:ui", ":core:domain")

private val coreAllowedDependencies = mapOf(
    ":core:model" to emptySet(),
    ":core:domain" to setOf(":core:model"),
    ":core:icp" to setOf(":core:model", ":core:domain"),
    ":core:verify" to setOf(":core:model", ":core:domain", ":core:icp"),
    ":core:platform" to setOf(":core:model", ":core:domain"),
)

internal fun allowedProjectDependencies(modulePath: String): Set<String>? =
    coreAllowedDependencies[modulePath] ?: featureAllowedDependencies.takeIf { modulePath.startsWith(":feature:") }

internal fun Project.enforceModuleRules() {
    val owner = path
    val allowed = allowedProjectDependencies(owner) ?: return
    configurations.configureEach {
        dependencies.withType<ProjectDependency>().configureEach {
            check(path == owner || path in allowed) { "Module rule violated: $owner may not depend on $path. Allowed: $allowed" }
        }
    }
}

internal fun Project.configureKmpAndroid() {
    pluginManager.apply("org.jetbrains.kotlin.multiplatform")
    pluginManager.apply("com.android.kotlin.multiplatform.library")
    enforceModuleRules()
    val catalog = libs
    val moduleNamespace = witnessNamespace(path)
    kmp.apply {
        androidLibrary {
            namespace = moduleNamespace
            compileSdk { version = release(catalog.sdk("compileSdk")) { minorApiLevel = catalog.sdk("compileSdkMinor") } }
            minSdk = catalog.sdk("minSdk")
            withHostTest {}
            compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
        }
        compilerOptions { freeCompilerArgs.add("-Xexpect-actual-classes") }
        sourceSets.commonTest.dependencies {
            implementation(catalog.lib("kotlin-test"))
            implementation(catalog.lib("kotlinx-coroutines-test"))
            implementation(catalog.lib("turbine"))
        }
    }
}
