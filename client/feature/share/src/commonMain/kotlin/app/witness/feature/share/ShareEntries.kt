package app.witness.feature.share

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.window.DialogProperties
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.scene.DialogSceneStrategy
import app.witness.core.domain.Clock
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.model.LocalCaseId
import kotlinx.serialization.Serializable

@Serializable
data class ShareCaseKey(val caseId: Long) : NavKey

@Serializable
data class ExportCaseKey(val caseId: Long) : NavKey

interface ShareNavigator {
    fun back()

    fun exportCase(id: LocalCaseId)

    fun sendLink(url: String)

    fun copyLink(url: String)

    fun preview(url: String)

    fun exported(result: ExportResult.Saved)

    fun upgrade()
}

class ShareDeps(
    val cases: CaseStore,
    val captures: CaptureStore,
    val shareCase: ShareCase,
    val entitlements: EntitlementSource,
    val exporter: CaseExporter,
    val clock: Clock,
    val detector: BlurDetector? = null,
    val redactor: ShareRedactor? = null,
    val offset: UtcOffset = UtcOffset.UTC,
    val image: RecordImage = {},
)

val ExportSheetMetadata: Map<String, Any> = DialogSceneStrategy.dialog(DialogProperties(usePlatformDefaultWidth = false))

fun EntryProviderScope<NavKey>.shareEntries(deps: ShareDeps, navigator: ShareNavigator) {
    entry<ShareCaseKey> { key -> ShareCaseRoute(LocalCaseId(key.caseId), deps, navigator) }
    entry<ExportCaseKey>(metadata = ExportSheetMetadata) { key -> ExportRoute(LocalCaseId(key.caseId), deps, navigator) }
}

@Composable
fun ShareCaseRoute(id: LocalCaseId, deps: ShareDeps, navigator: ShareNavigator) {
    val vm = viewModel(key = "share-${id.value}") {
        ShareCaseViewModel(id, deps.cases, deps.captures, deps.shareCase, deps.detector, deps.redactor, deps.clock, deps.offset)
    }
    val state by vm.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.missing) { if (state.missing) navigator.back() }
    LaunchedEffect(vm) { vm.effects.collect { if (it is ShareEvent.SendLink) navigator.sendLink(it.url) } }
    ShareCaseScreen(
        state = state,
        actions = ShareCaseActions(
            onClose = navigator::back,
            onToggleItem = vm::toggleItem,
            onTogglePrivacy = vm::togglePrivacy,
            onExpiry = vm::setExpiry,
            onIncludeSummary = vm::setIncludeSummary,
            onCopyLink = navigator::copyLink,
            onPreview = navigator::preview,
            onExportFile = { navigator.exportCase(id) },
            onShareLink = vm::shareLink,
            onClearError = vm::clearError,
        ),
        image = deps.image,
    )
}

@Composable
fun ExportRoute(id: LocalCaseId, deps: ShareDeps, navigator: ShareNavigator) {
    val vm = viewModel(key = "export-${id.value}") { ExportViewModel(id, deps.cases, deps.captures, deps.entitlements, deps.exporter, deps.clock) }
    val state by vm.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.missing) { if (state.missing) navigator.back() }
    LaunchedEffect(vm) {
        vm.effects.collect { event ->
            when (event) {
                is ExportEvent.Exported -> navigator.exported(event.result)
                ExportEvent.Upgrade -> navigator.upgrade()
            }
        }
    }
    SheetDialog(navigator::back) { ExportContent(state, vm::select, vm::setExactLocation, vm::export) }
}
