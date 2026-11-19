package app.witness.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.model.Tier
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

enum class PlusBusy { Purchasing, Restoring }

data class PlusUiState(
    val monthly: PlanOffer = PlanOffer.MONTHLY,
    val yearly: PlanOffer = PlanOffer.YEARLY,
    val selected: PlusPlan = PlusPlan.YEARLY,
    val active: Boolean = false,
    val account: Principal? = null,
    val busy: PlusBusy? = null,
    val notice: Notice? = null,
) {
    val savings: String? get() = SettingsCopy.savings(PlanMath.savingsPercent(monthly.priceMicros, yearly.priceMicros))
    val yearlyDetail: String get() = SettingsCopy.perYear(yearly)
}

sealed interface PlusEffect {
    data object LinkIdentity : PlusEffect

    data object ManageSubscription : PlusEffect
}

private data class PlusLocal(
    val offers: List<PlanOffer> = PlanOffer.DEFAULTS,
    val selected: PlusPlan = PlusPlan.YEARLY,
    val busy: PlusBusy? = null,
    val notice: Notice? = null,
)

class PlusViewModel(
    private val billing: PlusBilling,
    entitlements: EntitlementSource,
    private val identity: IdentityAccount,
    private val clock: Clock,
) : ViewModel() {
    private val local = MutableStateFlow(PlusLocal())
    private val effects = Channel<PlusEffect>(Channel.BUFFERED)

    val effect: Flow<PlusEffect> = effects.receiveAsFlow()

    val state: StateFlow<PlusUiState> = combine(local, entitlements.entitlement(), identity.principal) { l, e, principal ->
        PlusUiState(
            monthly = PlanMath.offer(l.offers, PlusPlan.MONTHLY),
            yearly = PlanMath.offer(l.offers, PlusPlan.YEARLY),
            selected = l.selected,
            active = e.effectiveTier(clock.nowEpochMillis()) == Tier.PLUS,
            account = principal,
            busy = l.busy,
            notice = l.notice,
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), PlusUiState())

    init {
        viewModelScope.launch {
            val offers = attempt { billing.offers() }.getOrNull()?.takeIf { it.isNotEmpty() } ?: return@launch
            local.update { it.copy(offers = offers) }
        }
    }

    fun select(plan: PlusPlan) = local.update { it.copy(selected = plan) }

    fun subscribe() {
        if (!claim(PlusBusy.Purchasing)) return
        viewModelScope.launch {
            val account = identity.principal.first()
            if (account == null) {
                local.update { it.copy(busy = null) }
                effects.send(PlusEffect.LinkIdentity)
                return@launch
            }
            finish(attempt { billing.purchase(local.value.selected, account) })
        }
    }

    fun restore() {
        if (!claim(PlusBusy.Restoring)) return
        viewModelScope.launch { finish(attempt { billing.restore(identity.principal.first()) }) }
    }

    fun manage() {
        effects.trySend(PlusEffect.ManageSubscription)
    }

    fun linkIdentity() {
        effects.trySend(PlusEffect.LinkIdentity)
    }

    fun dismissNotice() = local.update { it.copy(notice = null) }

    private fun claim(busy: PlusBusy): Boolean {
        var claimed = false
        local.update {
            claimed = it.busy == null
            if (claimed) it.copy(busy = busy, notice = null) else it
        }
        return claimed
    }

    private fun finish(result: Result<PurchaseOutcome>) {
        val outcome = result.getOrElse { PurchaseOutcome.Failed(it.message ?: it::class.simpleName.orEmpty()) }
        local.update { it.copy(busy = null, notice = SettingsCopy.purchaseNotice(outcome)) }
    }
}
