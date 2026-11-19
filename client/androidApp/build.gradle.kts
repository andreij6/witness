import com.android.build.api.artifact.SingleArtifact
import com.android.build.api.variant.BuildConfigField
import groovy.json.JsonSlurper

plugins {
    id("witness.android.app")
    id("witness.metro")
    alias(libs.plugins.baselineprofile)
}

android {
    namespace = "app.witness"
    defaultConfig {
        applicationId = "app.witness"
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    buildFeatures {
        buildConfig = true
    }
    testOptions {
        unitTests.isIncludeAndroidResources = true
    }
}

abstract class CleartextCheck : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val manifest: RegularFileProperty

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val configs: ConfigurableFileCollection

    @TaskAction
    fun verify() {
        val permits = Regex("""cleartextTrafficPermitted\s*=\s*["']true["']""")
        val offenders = configs.files.filter { permits.containsMatchIn(it.readText()) }.map { it.path }.toMutableList()
        val merged = manifest.get().asFile
        if (Regex("""usesCleartextTraffic\s*=\s*["']true["']""").containsMatchIn(merged.readText())) offenders += merged.path
        if (offenders.isNotEmpty()) throw GradleException("Release build allows cleartext traffic: ${offenders.joinToString()}")
    }
}

val icpIds = providers.fileContents(rootProject.layout.projectDirectory.file("../.icp-local/ids.json")).asText
fun canister(name: String, field: String) = icpIds.map { json ->
    val canisters = (JsonSlurper().parseText(json) as? Map<*, *>)?.get("canisters") as? Map<*, *>
    ((canisters?.get(name) as? Map<*, *>)?.get(field) as? String).orEmpty()
}.orElse("")
val localRegistry = canister("registry", "id")
val localPages = canister("pages", "url").map { it.trimEnd('/') }
val localAnchor = icpIds.map { json -> ((JsonSlurper().parseText(json) as? Map<*, *>)?.get("ca_root") as? String).orEmpty().lowercase() }.orElse("")
val releaseAnchor = providers.gradleProperty("witness.caRoot").orElse(providers.environmentVariable("WITNESS_CA_ROOT")).map { it.trim().lowercase() }.orElse("")
val localReplica = providers.gradleProperty("witness.replicaUrl").orElse("http://127.0.0.1:8090")

androidComponents {
    onVariants { variant ->
        val local = variant.buildType == "debug" || variant.name.contains("benchmark", ignoreCase = true)
        if (local) variant.sources.res?.addStaticSourceDirectory("src/local/res")
        variant.buildConfigFields?.apply {
            put("ICP_REPLICA_URL", localReplica.map { BuildConfigField("String", "\"${if (local) it else ""}\"", null) })
            put("ICP_REGISTRY_ID", localRegistry.map { BuildConfigField("String", "\"${if (local) it else ""}\"", null) })
            put("ICP_FETCH_ROOT_KEY", BuildConfigField("boolean", local.toString(), null))
            put("PAGES_ORIGIN", localPages.map { BuildConfigField("String", "\"${if (local) it else ""}\"", null) })
            put("WITNESS_CA_ROOT", (if (local) localAnchor else releaseAnchor).map { anchor ->
                require(anchor.matches(Regex("([0-9a-f]{2})*"))) { "witness CA root must be hex DER" }
                BuildConfigField("String", "\"$anchor\"", null)
            })
        }
        if (!local) {
            val name = variant.name.replaceFirstChar(Char::uppercaseChar)
            val check = tasks.register<CleartextCheck>("verify${name}Cleartext") {
                manifest.set(variant.artifacts.get(SingleArtifact.MERGED_MANIFEST))
                variant.sources.res?.all?.let { dirs -> configs.from(dirs.map { all -> all.flatten().map { d -> fileTree(d) { include("xml*/*.xml") } } }) }
            }
            tasks.named { it == "check" || it == "test" }.configureEach { dependsOn(check) }
        }
    }
}

dependencies {
    implementation(projects.core.designsystem)
    implementation(projects.core.domain)
    implementation(projects.core.ui)
    implementation(projects.core.telemetry)
    implementation(projects.core.data)
    implementation(projects.core.platform)
    implementation(projects.core.a2ui)
    implementation(projects.core.verify)
    implementation(projects.core.icp)
    implementation(projects.native.witness)
    implementation(projects.feature.home)
    implementation(projects.feature.capture)
    implementation(projects.feature.post)
    implementation(projects.feature.case)
    implementation(projects.feature.share)
    implementation(projects.feature.settings)
    implementation(projects.feature.check)
    implementation(projects.feature.proof)
    implementation(libs.jetbrains.navigation3.ui)
    implementation(libs.jetbrains.lifecycle.viewmodel.navigation3)
    implementation(libs.jetbrains.lifecycle.viewmodel.compose)
    implementation(libs.jetbrains.lifecycle.runtime.compose)
    implementation(libs.compose.runtime)
    implementation(libs.compose.foundation)
    implementation(libs.compose.ui)
    implementation(libs.compose.material3)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.core.splashscreen)
    implementation(libs.androidx.work.runtime)
    implementation(libs.androidx.camera.core)
    implementation(libs.androidx.camera.lifecycle)
    implementation(libs.androidx.profileinstaller)
    baselineProfile(projects.androidBenchmark)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.turbine)
}
