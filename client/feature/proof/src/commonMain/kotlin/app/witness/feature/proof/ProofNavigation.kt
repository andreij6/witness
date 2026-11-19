package app.witness.feature.proof

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.model.RecordId
import app.witness.core.ui.WebLink
import kotlinx.serialization.Serializable
import kotlinx.serialization.Transient

@Serializable
data class ItemProofKey(val recordId: String) : NavKey

@Serializable
data class TechnicalRecordKey(val recordId: String) : NavKey

@Serializable
data class ProveLocationKey(val recordId: String) : NavKey

@Serializable
data class ProofPageKey(val recordId: String) : NavKey

@Serializable
data class CaseLinkKey(val caseId: String, @Transient val fragment: String = "") : NavKey {
    override fun toString(): String = "CaseLinkKey(caseId=$caseId)"
}

@Serializable
data class ReportKey(val page: String) : NavKey

class ItemNavigation(
    val back: () -> Unit,
    val navigate: (NavKey) -> Unit,
    val share: (RecordId) -> Unit = {},
    val createPost: (RecordId) -> Unit = {},
    val shareFile: (name: String, mime: String, bytes: ByteArray) -> Unit = { _, _, _ -> },
)

class WebProofDependencies(
    val registry: RegistryGateway,
    val media: MediaGateway,
    val checkImage: CheckImage,
    val caseLinks: CaseLinkOpener,
    val reports: ReportSender,
    val time: TimeText = TimeText.UTC,
    val links: Links = Links(),
)

class WebProofNavigation(
    val openCheck: () -> Unit,
    val navigate: (NavKey) -> Unit,
)

fun EntryProviderScope<NavKey>.itemProofEntries(deps: ItemDependencies, navigation: ItemNavigation) {
    entry<ItemProofKey> { key -> RecordId.parseOrNull(key.recordId)?.let { ItemProofRoute(it, deps, navigation) } }
    entry<TechnicalRecordKey> { key -> RecordId.parseOrNull(key.recordId)?.let { TechnicalRecordRoute(it, deps, navigation) } }
    entry<ProveLocationKey> { key -> RecordId.parseOrNull(key.recordId)?.let { ProveLocationRoute(it, deps, navigation) } }
}

fun EntryProviderScope<NavKey>.webProofEntries(deps: WebProofDependencies, navigation: WebProofNavigation) {
    entry<ProofPageKey> { key -> ProofPageRoute(key.recordId, deps, navigation) }
    entry<CaseLinkKey> { key -> CaseLinkRoute(key.caseId, key.fragment, deps, navigation) }
    entry<ReportKey> { key -> ReportRoute(key.page, deps, navigation) }
}

@Composable
private fun itemViewModel(recordId: RecordId, deps: ItemDependencies, screen: String): ItemViewModel =
    rememberViewModel("item/$screen/${recordId.text}") { ItemViewModel(recordId, deps) }

@Suppress("DEPRECATION")
@Composable
private fun itemActions(vm: ItemViewModel, recordId: RecordId, navigation: ItemNavigation, onShowActions: (Boolean) -> Unit): ItemActions {
    val clipboard = LocalClipboardManager.current
    val uri = LocalUriHandler.current
    return remember(vm, navigation) {
        ItemActions(
            onBack = navigation.back,
            onShowActions = { onShowActions(true) },
            onHideActions = { onShowActions(false) },
            onTechnical = { onShowActions(false); navigation.navigate(TechnicalRecordKey(recordId.text)) },
            onShare = { navigation.share(recordId) },
            onMove = { onShowActions(false); vm.requestMove() },
            onCreatePost = { onShowActions(false); navigation.createPost(recordId) },
            onProveLocation = { onShowActions(false); navigation.navigate(ProveLocationKey(recordId.text)) },
            onDelete = { onShowActions(false); vm.requestDelete() },
            onErase = { onShowActions(false); vm.requestErase() },
            onConfirmErase = vm::confirmErase,
            onDismissErase = vm::dismissErase,
            onPickCase = vm::pickCase,
            onConfirmMove = vm::confirmMove,
            onDismissMove = vm::dismissMove,
            onConfirmDelete = vm::confirmDelete,
            onDismissDelete = vm::dismissDelete,
            onCopy = { clipboard.setText(AnnotatedString(it)) },
            onOpenWeb = { vm.state.value.webUrl?.let(uri::openUri) },
            onCreateLocationProof = {
                vm.state.value.location?.let { navigation.shareFile(locationProofName(recordId), "application/json", it.file.encodeToByteArray()) }
            },
        )
    }
}

@Composable
fun ItemProofRoute(recordId: RecordId, deps: ItemDependencies, navigation: ItemNavigation, modifier: Modifier = Modifier) {
    val vm = itemViewModel(recordId, deps, "proof")
    val state by vm.state.collectAsState()
    var sheet by remember { mutableStateOf(false) }
    LaunchedEffect(state.delete) { if (state.delete == DeleteState.Deleted) navigation.back() }
    ItemProofScreen(state, itemActions(vm, recordId, navigation) { sheet = it }, sheet, modifier)
}

@Composable
fun TechnicalRecordRoute(recordId: RecordId, deps: ItemDependencies, navigation: ItemNavigation, modifier: Modifier = Modifier) {
    val vm = itemViewModel(recordId, deps, "technical")
    val state by vm.state.collectAsState()
    TechnicalRecordScreen(state, itemActions(vm, recordId, navigation) {}, modifier)
}

@Composable
fun ProveLocationRoute(recordId: RecordId, deps: ItemDependencies, navigation: ItemNavigation, modifier: Modifier = Modifier) {
    val vm = itemViewModel(recordId, deps, "location")
    val state by vm.state.collectAsState()
    ProveLocationScreen(state, itemActions(vm, recordId, navigation) {}, modifier)
}

@Composable
private fun webActions(navigation: WebProofNavigation, page: String?, onRetry: () -> Unit = {}, onOpenItem: (Int) -> Unit = {}): WebActions {
    val uri = LocalUriHandler.current
    return remember(navigation, page, uri) {
        val report = { page?.let { navigation.navigate(ReportKey(it)) } ?: Unit }
        WebActions(
            onCheckCopy = navigation.openCheck,
            onReport = report,
            onRetry = onRetry,
            onOpenItem = onOpenItem,
            footer = buildList {
                if (page != null) add(WebLink("Report this page", report, WitnessIcon.Report))
                add(WebLink("Contact", { uri.openUri(CONTACT_URL) }))
                add(WebLink("Check a copy", navigation.openCheck))
            },
            header = listOf(
                WebLink("Check a copy", navigation.openCheck),
                WebLink("About", { uri.openUri(ABOUT_URL) }),
                WebLink("Report", report),
            ),
        )
    }
}

@Composable
fun ProofPageRoute(recordId: String, deps: WebProofDependencies, navigation: WebProofNavigation, modifier: Modifier = Modifier) {
    val vm = rememberViewModel("proof/$recordId") { ProofPageViewModel(recordId, deps.registry, deps.media, deps.time) }
    val state by vm.state.collectAsState()
    ProofPageScreen(state, webActions(navigation, recordId, vm::retry), modifier)
}

@Composable
fun CaseLinkRoute(caseId: String, fragment: String, deps: WebProofDependencies, navigation: WebProofNavigation, modifier: Modifier = Modifier) {
    val vm = rememberViewModel("case/$caseId") { CaseLinkViewModel(caseId, fragment, deps.caseLinks, deps.checkImage, deps.time) }
    val state by vm.state.collectAsState()
    val onOpenItem: (Int) -> Unit = { i ->
        ((vm.state.value as? CaseLinkState.Open)?.items?.getOrNull(i)?.recordId)?.let { navigation.navigate(ProofPageKey(it.text)) }
    }
    CaseLinkScreen(state, webActions(navigation, "v/$caseId", onOpenItem = onOpenItem), modifier)
}

@Composable
fun ReportRoute(page: String, deps: WebProofDependencies, navigation: WebProofNavigation, modifier: Modifier = Modifier) {
    val vm = rememberViewModel("report/$page") { ReportViewModel(page, deps.reports) }
    val state by vm.state.collectAsState()
    val actions = remember(vm) { ReportActions(vm::select, vm::onDetails, vm::onEmail, vm::send) }
    ReportScreen(state, actions, webActions(navigation, null), modifier)
}

const val ABOUT_URL: String = "https://witness.app/about"
const val CONTACT_URL: String = "https://witness.app/contact"
