package app.witness

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.LifecycleStartEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.navigation3.rememberViewModelStoreNavEntryDecorator
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.runtime.rememberNavBackStack
import androidx.navigation3.runtime.rememberSaveableStateHolderNavEntryDecorator
import androidx.navigation3.scene.DialogSceneStrategy
import androidx.navigation3.ui.NavDisplay
import app.witness.core.designsystem.NavItem
import app.witness.core.designsystem.WitnessBottomNav
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.CaptureMode
import app.witness.di.Features
import app.witness.feature.capture.CameraKey
import app.witness.feature.capture.CameraRoute
import app.witness.feature.cases.CaseCameraPillsLayer
import app.witness.feature.cases.CaseCoachLayer
import app.witness.feature.cases.caseEntries
import app.witness.feature.check.CheckKey
import app.witness.feature.check.CheckRoute
import app.witness.feature.check.CheckSharedKey
import app.witness.feature.home.HomeKey
import app.witness.feature.home.homeEntries
import app.witness.feature.post.postEntries
import app.witness.feature.proof.itemProofEntries
import app.witness.feature.proof.webProofEntries
import app.witness.feature.settings.settingsEntries
import app.witness.feature.share.shareEntries
import app.witness.nav.AppNavigator
import app.witness.nav.NavKeys
import app.witness.nav.Outside
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

@Composable
fun WitnessApp(features: Features, outside: Outside, links: Flow<NavKey>) {
    val backStack = rememberNavBackStack(NavKeys.configuration, HomeKey.Splash)
    val navigator = remember(backStack) { AppNavigator(backStack, outside, features.links) }
    LaunchedEffect(navigator) { links.collect(navigator::open) }
    val provider = remember(navigator) { entryProvider { appEntries(features, navigator, outside) } }
    WitnessTheme {
        NavDisplay(
            backStack = backStack,
            onBack = navigator::back,
            entryDecorators = listOf(rememberSaveableStateHolderNavEntryDecorator(), rememberViewModelStoreNavEntryDecorator()),
            sceneStrategies = listOf(DialogSceneStrategy()),
            entryProvider = provider,
        )
    }
}

fun EntryProviderScope<NavKey>.appEntries(f: Features, nav: AppNavigator, outside: Outside) {
    homeEntries(f.homeDeps, nav.home)
    entry<CameraKey> { CameraEntry(f, nav) }
    postEntries(f::composer, f::shared, nav::back, { text, post -> outside.sharePost(text, post.recordId) }, nav::shared, nav::postDone)
    caseEntries(f.caseDeps, nav)
    shareEntries(f.shareDeps, nav)
    settingsEntries(f.settingsDeps, nav)
    entry<CheckKey> { CheckTab(f, nav) }
    entry<CheckSharedKey> { key -> CheckRoute(f.checkDeps, nav.check, sharedId = key.sharedId) }
    itemProofEntries(f.itemDeps, nav.item)
    webProofEntries(f.proofDeps, nav.proof)
}

@Composable
private fun CameraEntry(f: Features, nav: AppNavigator) {
    val mode by remember(f) { f.settingsStore.settings.map { it.captureMode } }.collectAsStateWithLifecycle(CaptureMode.POST)
    LifecycleStartEffect(f.gnss) {
        f.gnss.open()
        onStopOrDispose { f.gnss.close() }
    }
    CameraRoute(
        viewModel { f.camera() },
        remember(f) { listOf(f.captureSource.imageCapture) },
        onClose = nav::back,
        onCompose = nav::composed,
        onOpenItem = nav::openItem,
        analysis = f.liveBlur,
        topAccessory = { if (mode == CaptureMode.CASE) CaseCameraPillsLayer(f.caseDeps, nav) },
        bottomAccessory = { if (mode == CaptureMode.CASE) CaseCoachLayer(f.caseDeps) },
    )
}

private val Tabs = NavItem("Home", WitnessIcon.Home) to NavItem("Check", WitnessIcon.VerifiedUser)

@Composable
private fun CheckTab(f: Features, nav: AppNavigator) {
    CheckRoute(f.checkDeps, nav.check, Modifier.fillMaxSize()) {
        WitnessBottomNav(Tabs.first, Tabs.second, 1, { if (it == 0) nav.selectTab(HomeKey.Home) }, { nav.push(CameraKey) })
    }
}
