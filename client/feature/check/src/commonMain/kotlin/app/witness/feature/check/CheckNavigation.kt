package app.witness.feature.check

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.designsystem.Witness
import app.witness.core.model.CaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.WebLink
import kotlinx.serialization.Serializable

@Serializable
data object CheckKey : NavKey

@Serializable
data class CheckSharedKey(val sharedId: String) : NavKey

class CheckNavigation(
    val openProof: (RecordId) -> Unit,
    val openCase: (CaseId, String) -> Unit,
    val close: () -> Unit = {},
    val share: (String) -> Unit = {},
)

fun EntryProviderScope<NavKey>.checkEntries(deps: CheckDependencies, navigation: CheckNavigation) {
    entry<CheckKey> { CheckRoute(deps, navigation) }
    entry<CheckSharedKey> { key -> CheckRoute(deps, navigation, sharedId = key.sharedId) }
}

@Composable
fun CheckRoute(
    deps: CheckDependencies,
    navigation: CheckNavigation,
    modifier: Modifier = Modifier,
    sharedId: String? = null,
    idleFooter: @Composable () -> Unit = {},
) {
    val vm = rememberViewModel("check/${sharedId.orEmpty()}") {
        CheckViewModel(deps, CheckSurface.App, if (sharedId != null) "from another app" else null)
    }
    if (sharedId != null) LaunchedEffect(sharedId) { vm.checkShared(sharedId) }
    val state by vm.state.collectAsState()
    val picker = rememberImagePicker(vm::check)
    val actions = remember(vm, navigation) { actionsFor(vm, navigation, picker) }
    Column(modifier.background(Witness.colors.paper)) {
        CheckScreen(state, actions, Modifier.weight(1f), shared = sharedId != null, now = deps.clock.nowEpochMillis(), time = deps.time)
        if (state.phase == CheckPhase.Idle) idleFooter()
    }
}

@Composable
fun WebCheckRoute(deps: CheckDependencies, navigation: CheckNavigation, modifier: Modifier = Modifier) {
    val vm = rememberViewModel("check/web") { CheckViewModel(deps, CheckSurface.Web) }
    val state by vm.state.collectAsState()
    val picker = rememberImagePicker(vm::check)
    val actions = remember(vm, navigation) { actionsFor(vm, navigation, picker) }
    val uri = LocalUriHandler.current
    val footer = remember(uri) { siteLinks { uri.openUri(it) } }
    WebCheckPage(state, actions, footer, modifier, headerLinks = footer)
}

fun siteLinks(open: (String) -> Unit): List<WebLink> = listOf(
    WebLink("About", { open(ABOUT_URL) }),
    WebLink("Contact", { open(CONTACT_URL) }),
    WebLink("Terms", { open(TERMS_URL) }),
)

const val ABOUT_URL: String = "https://witness.app/about"
const val CONTACT_URL: String = "https://witness.app/contact"
const val TERMS_URL: String = "https://witness.app/terms"

internal fun actionsFor(vm: CheckViewModel, navigation: CheckNavigation, picker: ImagePicker): CheckActions = CheckActions(
    onChoose = picker::launch,
    onLinkChange = vm::onLinkChange,
    onSubmitLink = { text ->
        when (val target = vm.submitLink(text)) {
            is PastedLink.Proof -> navigation.openProof(target.recordId)
            is PastedLink.Case -> navigation.openCase(target.caseId, target.fragment)
            else -> Unit
        }
    },
    onCancel = vm::cancel,
    onOpenProof = navigation.openProof,
    onShare = navigation.share,
    onNotify = vm::setNotifyWhenAnchored,
    onClose = navigation.close,
)

@Composable
internal inline fun <reified VM : ViewModel> rememberViewModel(key: String, noinline create: () -> VM): VM {
    val owner = LocalViewModelStoreOwner.current ?: rememberLocalOwner()
    return viewModel(owner, key) { create() }
}

@Composable
@PublishedApi
internal fun rememberLocalOwner(): ViewModelStoreOwner {
    val owner = remember { object : ViewModelStoreOwner { override val viewModelStore = ViewModelStore() } }
    DisposableEffect(owner) { onDispose { owner.viewModelStore.clear() } }
    return owner
}
