package app.witness.feature.cases

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
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import kotlinx.serialization.Serializable

@Serializable
data object CasesKey : NavKey

@Serializable
data class CaseKey(val caseId: Long) : NavKey

@Serializable
data object CaseSwitcherKey : NavKey

@Serializable
data object NewCaseKey : NavKey

interface CaseNavigator {
    fun back()

    fun openCase(id: LocalCaseId)

    fun newCase()

    fun switchCase()

    fun caseCreated(id: LocalCaseId)

    fun shareCase(id: LocalCaseId)

    fun exportCase(id: LocalCaseId)

    fun captureFor(id: LocalCaseId, target: String?)

    fun openItem(recordId: RecordId)

    fun search()
}

class CaseDeps(
    val cases: CaseStore,
    val captures: CaptureStore,
    val settings: SettingsStore,
    val manageCases: ManageCases,
    val catalog: TemplateCatalog,
    val feedback: AiFeedback,
    val clock: Clock,
    val offset: UtcOffset = UtcOffset.UTC,
    val image: RecordImage = {},
    val report: CaseReportSlot? = null,
)

val SheetDialogMetadata: Map<String, Any> = DialogSceneStrategy.dialog(DialogProperties(usePlatformDefaultWidth = false))

fun EntryProviderScope<NavKey>.caseEntries(deps: CaseDeps, navigator: CaseNavigator) {
    entry<CasesKey> { CasesRoute(deps, navigator) }
    entry<CaseKey> { key -> CaseRoute(LocalCaseId(key.caseId), deps, navigator) }
    entry<CaseSwitcherKey>(metadata = SheetDialogMetadata) { CaseSwitcherRoute(deps, navigator) }
    entry<NewCaseKey>(metadata = SheetDialogMetadata) { NewCaseRoute(deps, navigator) }
}

@Composable
fun CasesRoute(deps: CaseDeps, navigator: CaseNavigator) {
    val vm = viewModel(key = "cases") { CasesViewModel(deps.cases, deps.captures, deps.clock, deps.offset) }
    val state by vm.state.collectAsStateWithLifecycle()
    CasesScreen(
        state = state,
        onFilter = vm::setFilter,
        onOpen = { navigator.openCase(it.id) },
        onNewCase = navigator::newCase,
        onBack = navigator::back,
        onSearch = navigator::search,
        image = deps.image,
    )
}

@Composable
fun CaseRoute(id: LocalCaseId, deps: CaseDeps, navigator: CaseNavigator) {
    val vm = viewModel(key = "case-${id.value}") {
        CaseReportViewModel(id, deps.cases, deps.captures, deps.manageCases, deps.catalog, deps.feedback, deps.clock, deps.offset)
    }
    val state by vm.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.missing) { if (state.missing) navigator.back() }
    CaseReportScreen(
        state = state,
        actions = vm.actions(
            onBack = navigator::back,
            onOpenItem = navigator::openItem,
            onCapture = { navigator.captureFor(id, it) },
            onExport = { navigator.exportCase(id) },
            onShare = { navigator.shareCase(id) },
        ),
        image = deps.image,
        report = deps.report,
    )
}

@Composable
fun CaseSwitcherRoute(deps: CaseDeps, navigator: CaseNavigator) {
    val vm = viewModel(key = "case-switcher") { CaseSwitcherViewModel(deps.cases, deps.captures, deps.settings, deps.manageCases) }
    val state by vm.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.done) { if (state.done) navigator.back() }
    SheetDialog(navigator::back) { CaseSwitcherContent(state, vm::select, navigator::newCase) }
}

@Composable
fun NewCaseRoute(deps: CaseDeps, navigator: CaseNavigator) {
    val vm = viewModel(key = "new-case") { NewCaseViewModel(deps.manageCases, deps.catalog) }
    val state by vm.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.created) { state.created?.let(navigator::caseCreated) }
    SheetDialog(navigator::back) { NewCaseContent(state, vm::setName, vm::select, vm::create) }
}

@Composable
private fun caseCameraViewModel(deps: CaseDeps): CaseCameraViewModel =
    viewModel(key = "case-camera") { CaseCameraViewModel(deps.settings, deps.cases, deps.captures, deps.feedback, deps.clock) }

@Composable
fun CaseCameraPillsLayer(deps: CaseDeps, navigator: CaseNavigator) {
    val state by caseCameraViewModel(deps).state.collectAsStateWithLifecycle()
    if (state.visible) CaseCameraPills(state.pillLabel, navigator::switchCase, navigator::newCase)
}

@Composable
fun CaseCoachLayer(deps: CaseDeps) {
    val vm = caseCameraViewModel(deps)
    val state by vm.state.collectAsStateWithLifecycle()
    val coach = state.coach
    if (state.visible && coach != null) EvidenceCoachCard(coach, vm::skip, vm::flagCoach)
}
