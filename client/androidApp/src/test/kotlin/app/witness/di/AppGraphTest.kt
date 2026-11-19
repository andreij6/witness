package app.witness.di

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import app.witness.core.telemetry.Telemetry
import dev.zacsweers.metro.createGraphFactory
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertSame

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class AppGraphTest {
    private val graph: AppGraph = createGraphFactory<AppGraph.Factory>().create(ApplicationProvider.getApplicationContext())

    @Test
    fun telemetryIsNoOpWithoutFirebaseConfig() {
        assertSame(Telemetry.None, (graph.telemetry as? LocalLogTelemetry)?.inner ?: graph.telemetry)
    }

    @Test
    fun nativePortsAreTheStandInAndSingletons() {
        assertFalse(graph.native.linked)
        assertSame(graph.native, graph.native)
    }

    @Test
    fun scopedBindingsAreShared() {
        assertSame(graph.features, graph.features)
        assertSame(graph.vault, graph.vault)
        assertSame(graph.anchorOutbox, graph.anchorOutbox)
        assertSame(graph.captureSource, graph.captureSource)
    }

    @Test
    fun everyFeatureDependencyResolves() {
        val f = graph.features
        assertNotNull(f.homeDeps)
        assertNotNull(f.caseDeps)
        assertNotNull(f.shareDeps)
        assertNotNull(f.settingsDeps)
        assertNotNull(f.checkDeps)
        assertNotNull(f.itemDeps)
        assertNotNull(f.proofDeps)
        assertSame(f.homeDeps, f.homeDeps)
    }

    @Test
    fun featureDepsShareTheNativePorts() {
        val f = graph.features
        assertSame(graph.native.registry, f.itemDeps.registry)
        assertSame(graph.native.registry, f.proofDeps.registry)
        assertSame(graph.native.media, f.checkDeps.media)
    }
}
