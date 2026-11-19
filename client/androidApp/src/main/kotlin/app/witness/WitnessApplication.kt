package app.witness

import android.app.Application
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.work.Configuration
import app.witness.core.data.AnchorOutboxHost
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.di.AppGraph
import app.witness.di.LocalLogTelemetry
import dev.zacsweers.metro.createGraphFactory
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class WitnessApplication : Application(), AnchorOutboxHost, Configuration.Provider {
    val graph: AppGraph by lazy { createGraphFactory<AppGraph.Factory>().create(this) }

    val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override val anchorOutbox: AnchorOutbox get() = graph.anchorOutbox

    override val workManagerConfiguration: Configuration get() = Configuration.Builder().build()

    override fun onCreate() {
        super.onCreate()
        scope.launch {
            ProcessCameraProvider.getInstance(this@WitnessApplication)
            val g = graph
            LocalLogTelemetry.gateOf(g.telemetry)?.follow(g.settings, scope)
            g.native.restore(g.vault)
        }
    }
}
