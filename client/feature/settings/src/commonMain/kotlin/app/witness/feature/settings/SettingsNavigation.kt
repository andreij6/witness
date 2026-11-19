package app.witness.feature.settings

import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.domain.Clock
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.Capture
import app.witness.core.model.RecordId
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.serialization.Serializable

@Serializable
sealed interface SettingsKey : NavKey {
    @Serializable
    data object Root : SettingsKey

    @Serializable
    data object Plus : SettingsKey

    @Serializable
    data object LinkIdentity : SettingsKey

    @Serializable
    data object PrivacyData : SettingsKey
}

interface SettingsNavigator {
    fun navigate(key: SettingsKey)

    fun back()

    fun openExternal(url: String)
}

class SettingsDependencies(
    val settings: SettingsStore,
    val entitlements: EntitlementSource,
    val identity: IdentityAccount,
    val linkFlow: IdentityLinkFlow,
    val consent: TelemetryConsentStore,
    val assistant: Assistant,
    val billing: PlusBilling,
    val exporter: DataExporter,
    val eraseAll: suspend () -> EraseReport,
    val clock: Clock,
    val captures: () -> Flow<List<Capture>> = { emptyFlow() },
    val eraseItem: suspend (RecordId) -> EraseReport = { EraseReport() },
) {
    fun settingsViewModel(): SettingsViewModel =
        SettingsViewModel(settings, entitlements, identity, consent, assistant, billing, DeleteAccount(eraseAll, identity), clock)

    fun plusViewModel(): PlusViewModel = PlusViewModel(billing, entitlements, identity, clock)

    fun identityLinkViewModel(): IdentityLinkViewModel = IdentityLinkViewModel(linkFlow, identity)

    fun privacyDataViewModel(): PrivacyDataViewModel = PrivacyDataViewModel(exporter, captures(), eraseItem, eraseAll)
}

fun EntryProviderScope<NavKey>.settingsEntries(deps: SettingsDependencies, navigator: SettingsNavigator) {
    entry<SettingsKey.Root> { SettingsRoute(viewModel { deps.settingsViewModel() }, navigator) }
    entry<SettingsKey.Plus> { PlusRoute(viewModel { deps.plusViewModel() }, navigator) }
    entry<SettingsKey.LinkIdentity> { IdentityLinkRoute(viewModel { deps.identityLinkViewModel() }, navigator) }
    entry<SettingsKey.PrivacyData> { PrivacyDataRoute(viewModel { deps.privacyDataViewModel() }, navigator) }
}
