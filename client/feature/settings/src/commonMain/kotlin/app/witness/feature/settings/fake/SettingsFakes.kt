package app.witness.feature.settings.fake

import app.witness.core.domain.port.OfflineException
import app.witness.core.model.Entitlement
import app.witness.core.model.Tier
import app.witness.feature.settings.DataExporter
import app.witness.feature.settings.ExportOutcome
import app.witness.feature.settings.IdentityAccount
import app.witness.feature.settings.PlanOffer
import app.witness.feature.settings.PlusBilling
import app.witness.feature.settings.PlusPlan
import app.witness.feature.settings.Principal
import app.witness.feature.settings.PurchaseOutcome
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow

class InMemoryIdentityAccount(initial: Principal? = null, var accept: Boolean = true, var offline: Boolean = false) : IdentityAccount {
    private val state = MutableStateFlow(initial)
    var unlinkCount: Int = 0
        private set

    override val principal: Flow<Principal?> get() = state

    val value: Principal? get() = state.value

    override suspend fun link(principal: Principal): Boolean {
        if (offline) throw OfflineException()
        if (accept) state.value = principal
        return accept
    }

    override suspend fun unlink() {
        if (offline) throw OfflineException()
        unlinkCount++
        state.value = null
    }
}

class FakePlusBilling(
    var offers: List<PlanOffer> = PlanOffer.DEFAULTS,
    var outcome: PurchaseOutcome = PurchaseOutcome.Purchased,
    var restoreOutcome: PurchaseOutcome = PurchaseOutcome.NothingToRestore,
    var failure: Exception? = null,
    private val entitlement: MutableStateFlow<Entitlement>? = null,
    private val grantExpiresAtMs: Long? = null,
) : PlusBilling {
    val purchases = ArrayList<Pair<PlusPlan, Principal>>()
    val restores = ArrayList<Principal?>()

    override suspend fun offers(): List<PlanOffer> {
        failure?.let { throw it }
        return offers
    }

    override suspend fun purchase(plan: PlusPlan, account: Principal): PurchaseOutcome {
        failure?.let { throw it }
        purchases += plan to account
        return outcome.also(::grantIfPurchased)
    }

    override suspend fun restore(account: Principal?): PurchaseOutcome {
        failure?.let { throw it }
        restores += account
        return restoreOutcome.also(::grantIfPurchased)
    }

    private fun grantIfPurchased(outcome: PurchaseOutcome) {
        if (outcome == PurchaseOutcome.Purchased) entitlement?.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, grantExpiresAtMs)
    }
}

class FakeDataExporter(var outcome: ExportOutcome = ExportOutcome.Ready(0), var failure: Exception? = null) : DataExporter {
    var calls: Int = 0
        private set

    override suspend fun export(): ExportOutcome {
        calls++
        failure?.let { throw it }
        return outcome
    }
}
