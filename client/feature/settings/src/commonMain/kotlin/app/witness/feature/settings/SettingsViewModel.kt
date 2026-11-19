package app.witness.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Tier
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

enum class DeletionState { Hidden, Confirming, Running }

data class SettingsUiState(
    val account: Principal? = null,
    val plusActive: Boolean = false,
    val monthlyPrice: String = PlanOffer.MONTHLY.price,
    val precision: LocationPrecision = LocationPrecision.AREA,
    val aiEnabled: Boolean = true,
    val aiOnDevice: Boolean = false,
    val consent: TelemetryConsent = TelemetryConsent.NONE,
    val precisionPicker: Boolean = false,
    val deletion: DeletionState = DeletionState.Hidden,
    val notice: Notice? = null,
)

private data class SettingsLocal(
    val monthlyPrice: String = PlanOffer.MONTHLY.price,
    val aiOnDevice: Boolean = false,
    val precisionPicker: Boolean = false,
    val deletion: DeletionState = DeletionState.Hidden,
    val notice: Notice? = null,
)

class SettingsViewModel(
    private val settings: SettingsStore,
    entitlements: EntitlementSource,
    identity: IdentityAccount,
    private val consentStore: TelemetryConsentStore,
    private val assistant: Assistant,
    private val billing: PlusBilling,
    private val deleteAccount: DeleteAccount,
    private val clock: Clock,
) : ViewModel() {
    private val local = MutableStateFlow(SettingsLocal())

    val state: StateFlow<SettingsUiState> = combine(
        settings.settings,
        entitlements.entitlement(),
        identity.principal,
        consentStore.consent,
        local,
    ) { s, e, principal, consent, l ->
        SettingsUiState(
            account = principal,
            plusActive = e.effectiveTier(clock.nowEpochMillis()) == Tier.PLUS,
            monthlyPrice = l.monthlyPrice,
            precision = s.defaultPrecision,
            aiEnabled = s.aiAssistant,
            aiOnDevice = l.aiOnDevice,
            consent = consent,
            precisionPicker = l.precisionPicker,
            deletion = l.deletion,
            notice = l.notice,
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), SettingsUiState())

    init {
        viewModelScope.launch {
            val onDevice = attempt { assistant.isAvailable() }.getOrDefault(false)
            local.update { it.copy(aiOnDevice = onDevice) }
        }
        viewModelScope.launch {
            val price = attempt { PlanMath.offer(billing.offers(), PlusPlan.MONTHLY).price }.getOrNull() ?: return@launch
            local.update { it.copy(monthlyPrice = price) }
        }
    }

    fun setAiEnabled(enabled: Boolean) {
        viewModelScope.launch { settings.update { it.copy(aiAssistant = enabled) } }
    }

    fun openPrecisionPicker() = local.update { it.copy(precisionPicker = true) }

    fun closePrecisionPicker() = local.update { it.copy(precisionPicker = false) }

    fun setPrecision(precision: LocationPrecision) {
        closePrecisionPicker()
        viewModelScope.launch { settings.update { it.copy(defaultPrecision = precision) } }
    }

    fun setAnalytics(enabled: Boolean) = updateConsent { it.copy(analytics = enabled) }

    fun setCrashReports(enabled: Boolean) = updateConsent { it.copy(crashReports = enabled) }

    fun requestDelete() = local.update { if (it.deletion == DeletionState.Hidden) it.copy(deletion = DeletionState.Confirming) else it }

    fun cancelDelete() = local.update { if (it.deletion == DeletionState.Confirming) it.copy(deletion = DeletionState.Hidden) else it }

    fun confirmDelete() {
        var started = false
        local.update {
            started = it.deletion == DeletionState.Confirming
            if (started) it.copy(deletion = DeletionState.Running) else it
        }
        if (!started) return
        viewModelScope.launch {
            val notice = attempt { SettingsCopy.deleteNotice(deleteAccount()) }
                .getOrElse { SettingsCopy.deleteNotice(DeleteOutcome.Incomplete(EraseReport(offline = true))) }
            local.update { it.copy(deletion = DeletionState.Hidden, notice = notice) }
        }
    }

    fun dismissNotice() = local.update { it.copy(notice = null) }

    private fun updateConsent(transform: (TelemetryConsent) -> TelemetryConsent) {
        viewModelScope.launch { consentStore.set(transform(consentStore.consent.first())) }
    }

    private companion object {
        const val STOP_TIMEOUT_MS = 5_000L
    }
}
