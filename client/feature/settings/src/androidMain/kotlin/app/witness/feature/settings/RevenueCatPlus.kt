package app.witness.feature.settings

import app.witness.core.domain.port.EntitlementSource
import app.witness.core.model.Entitlement
import app.witness.core.model.Tier
import com.revenuecat.purchases.kmp.Purchases
import com.revenuecat.purchases.kmp.PurchasesDelegate
import com.revenuecat.purchases.kmp.ktx.awaitCustomerInfo
import com.revenuecat.purchases.kmp.ktx.awaitLogIn
import com.revenuecat.purchases.kmp.ktx.awaitOfferings
import com.revenuecat.purchases.kmp.ktx.awaitPurchase
import com.revenuecat.purchases.kmp.ktx.awaitRestore
import com.revenuecat.purchases.kmp.models.CustomerInfo
import com.revenuecat.purchases.kmp.models.Offering
import com.revenuecat.purchases.kmp.models.Package
import com.revenuecat.purchases.kmp.models.PurchasesError
import com.revenuecat.purchases.kmp.models.PurchasesException
import com.revenuecat.purchases.kmp.models.PurchasesTransactionException
import com.revenuecat.purchases.kmp.models.StoreProduct
import com.revenuecat.purchases.kmp.models.StoreTransaction
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flowOf

class RevenueCatPlus(private val entitlementId: String = ENTITLEMENT_ID) : PlusBilling, EntitlementSource {
    override fun entitlement(): Flow<Entitlement> {
        if (!Purchases.isConfigured) return flowOf(Entitlement.FREE)
        return callbackFlow {
            val purchases = Purchases.sharedInstance
            purchases.delegate = object : PurchasesDelegate {
                override fun onCustomerInfoUpdated(customerInfo: CustomerInfo) {
                    trySend(entitlementOf(customerInfo))
                }

                override fun onPurchasePromoProduct(
                    product: StoreProduct,
                    startPurchase: (onError: (PurchasesError, Boolean) -> Unit, onSuccess: (StoreTransaction, CustomerInfo) -> Unit) -> Unit,
                ) = Unit
            }
            attempt { purchases.awaitCustomerInfo() }.onSuccess { trySend(entitlementOf(it)) }
            awaitClose { purchases.delegate = null }
        }.distinctUntilChanged()
    }

    override suspend fun offers(): List<PlanOffer> {
        if (!Purchases.isConfigured) return emptyList()
        val current = Purchases.sharedInstance.awaitOfferings().current ?: return emptyList()
        return buildList(2) {
            current.monthly?.let { add(offerOf(PlusPlan.MONTHLY, it)) }
            current.annual?.let { add(offerOf(PlusPlan.YEARLY, it)) }
        }
    }

    override suspend fun purchase(plan: PlusPlan, account: Principal): PurchaseOutcome {
        if (!Purchases.isConfigured) return PurchaseOutcome.Failed(NOT_CONFIGURED)
        val purchases = Purchases.sharedInstance
        return try {
            purchases.awaitLogIn(account.text)
            val pkg = packageOf(purchases.awaitOfferings().current, plan) ?: return PurchaseOutcome.Failed("no ${plan.name.lowercase()} package")
            if (isEntitled(purchases.awaitPurchase(pkg).customerInfo)) PurchaseOutcome.Purchased else PurchaseOutcome.Failed("entitlement not granted")
        } catch (e: PurchasesTransactionException) {
            if (e.userCancelled) PurchaseOutcome.Cancelled else PurchaseOutcome.Failed(e.error.code.name)
        } catch (e: PurchasesException) {
            PurchaseOutcome.Failed(e.error.code.name)
        }
    }

    override suspend fun restore(account: Principal?): PurchaseOutcome {
        if (!Purchases.isConfigured) return PurchaseOutcome.Failed(NOT_CONFIGURED)
        val purchases = Purchases.sharedInstance
        return try {
            if (account != null) purchases.awaitLogIn(account.text)
            if (isEntitled(purchases.awaitRestore())) PurchaseOutcome.Purchased else PurchaseOutcome.NothingToRestore
        } catch (e: PurchasesException) {
            PurchaseOutcome.Failed(e.error.code.name)
        }
    }

    private fun isEntitled(info: CustomerInfo): Boolean = info.entitlements.active.containsKey(entitlementId)

    private fun entitlementOf(info: CustomerInfo): Entitlement {
        val active = info.entitlements.active[entitlementId] ?: return Entitlement.FREE
        return Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, active.expirationDateMillis)
    }

    private fun offerOf(plan: PlusPlan, pkg: Package): PlanOffer = PlanOffer(plan, pkg.storeProduct.price.formatted, pkg.storeProduct.price.amountMicros)

    private fun packageOf(offering: Offering?, plan: PlusPlan): Package? = when (plan) {
        PlusPlan.MONTHLY -> offering?.monthly
        PlusPlan.YEARLY -> offering?.annual
    }

    companion object {
        const val ENTITLEMENT_ID = "plus"
        const val NOT_CONFIGURED = "billing not configured"
    }
}
