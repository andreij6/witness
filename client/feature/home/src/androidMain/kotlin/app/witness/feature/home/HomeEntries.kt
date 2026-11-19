package app.witness.feature.home

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey

fun EntryProviderScope<NavKey>.homeEntries(deps: HomeDependencies, navigator: HomeNavigator) {
    entry<HomeKey.Splash> { SplashRoute(viewModel { SplashViewModel(deps.vault) }, navigator) }
    entry<HomeKey.Welcome> { WelcomeScreen(onGetStarted = { navigator.push(HomeKey.Permissions) }, onCheckPhoto = navigator.openCheck) }
    entry<HomeKey.Permissions> { PermissionsRoute(viewModel { PermissionsViewModel() }, navigator) }
    entry<HomeKey.DeviceSetup> { DeviceSetupRoute(viewModel { DeviceSetupViewModel(deps.vault, deps.enrollment) }, navigator) }
    entry<HomeKey.Unsupported> { UnsupportedScreen(onCheckPhoto = navigator.openCheck) }
    entry<HomeKey.Home> {
        WithThumbnails(deps) {
            val vm = viewModel { HomeViewModel(deps.captures, deps.cases, deps.vault, deps.time) }
            val state by vm.state.collectAsStateWithLifecycle()
            HomeScreen(
                state,
                HomeActions(
                    onFilter = vm::select,
                    onSearch = { navigator.push(HomeKey.Search) },
                    onSettings = navigator.openSettings,
                    onAnchoring = { navigator.push(HomeKey.Anchoring) },
                    onCamera = navigator.openCamera,
                    onCheck = navigator.openCheck,
                    onItem = navigator.openItem,
                    onCase = navigator.openCase,
                    onNewCase = navigator.newCase,
                ),
            )
        }
    }
    entry<HomeKey.Anchoring> {
        WithThumbnails(deps) {
            val vm = viewModel { AnchoringViewModel(deps.captures, deps.cases, deps.outbox, deps.rotation, deps.scheduler, deps.time) }
            val state by vm.state.collectAsStateWithLifecycle()
            AnchoringScreen(state, onBack = navigator.back, onRetry = vm::retry, onItem = navigator.openItem)
        }
    }
    entry<HomeKey.Search> {
        WithThumbnails(deps) {
            val vm = viewModel { SearchViewModel(deps.captures, deps.cases, deps.time) }
            val state by vm.state.collectAsStateWithLifecycle()
            val query by vm.query.collectAsStateWithLifecycle()
            SearchScreen(state, query, vm::onQuery, vm::select, navigator.back, navigator.openItem, navigator.openCase)
        }
    }
}

@Composable
private fun WithThumbnails(deps: HomeDependencies, content: @Composable () -> Unit) =
    CompositionLocalProvider(LocalThumbnails provides deps.thumbnails, content = content)
