package app.witness.core.a2ui

import androidx.a2ui.compose.runtime.A2uiComponentState
import androidx.a2ui.compose.runtime.LocalA2uiReadinessEvaluator
import androidx.a2ui.compose.runtime.observeA2uiComponentState
import androidx.a2ui.compose.ui.A2uiCatalog
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.a2ui.compose.ui.A2uiMessageProcessor
import androidx.a2ui.compose.ui.asReadinessEvaluator
import androidx.a2ui.engine.model.A2uiCoreSurfaceModel
import androidx.a2ui.model.processor.A2uiMessageProcessor
import androidx.a2ui.model.protocol.A2uiClientErrorMessage
import androidx.a2ui.model.protocol.A2uiDataPath
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.catalog.WitnessCatalogV1
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.serialization.json.JsonObject

class WitnessReport(sink: WitnessEventSink, val catalog: A2uiCatalog = WitnessCatalogV1.catalog) {
    val processor: A2uiMessageProcessor = A2uiMessageProcessor(listOf(catalog), listOf(WitnessActionInterceptor(sink)))

    val errors: Flow<A2uiClientErrorMessage> = processor.outboundEvents.filterIsInstance()

    suspend fun run() = processor.collectMessages()

    fun show(template: Template, binding: Binding? = null) {
        send(A2uiMessages.deleteSurface(template.surfaceId))
        for (message in template.messages) send(message)
        if (binding != null) send(binding.message)
    }

    fun send(message: JsonObject) = processor.processMessage(A2uiBridge.toLibrary(message))

    fun slots(surfaceId: String): Map<String, Any?> {
        val surface = processor.activeSurfaces.value.firstOrNull { it.id == surfaceId } as? A2uiCoreSurfaceModel
            ?: return emptyMap()
        val raw = surface.dataModel[slotsPath] as? Map<*, *> ?: return emptyMap()
        return HashMap<String, Any?>(raw.size * 2).apply { for ((key, value) in raw) put(key.toString(), value) }
    }

    private companion object {
        val slotsPath = A2uiDataPath(A2uiMessages.SLOTS_PATH)
    }
}

@Composable
fun WitnessReportSurface(report: WitnessReport, surfaceId: String, modifier: Modifier = Modifier) {
    val surfaces by report.processor.activeSurfaces.collectAsState()
    val surface = surfaces.firstOrNull { it.id == surfaceId } ?: return
    val evaluator = remember(report.catalog) { report.catalog.asReadinessEvaluator() }
    CompositionLocalProvider(LocalA2uiReadinessEvaluator provides evaluator) {
        val state = observeA2uiComponentState(surface)
        if (state is A2uiComponentState.Success) A2uiComponent(state.component, modifier)
    }
}
