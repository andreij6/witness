package app.witness.feature.settings

import app.witness.core.designsystem.Tone
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.LocationPrecision

data class Notice(val text: String, val tone: Tone)

object SettingsCopy {
    const val NO_ACCOUNT = "No account"
    const val NO_ACCOUNT_HINT = "Optional: sign in with Internet Identity to sync"
    const val LINKED_ACCOUNT = "Internet Identity"
    const val AI_ON_DEVICE = "Gemini Nano available"
    const val AI_RULES = "Rules-based suggestions on this phone"
    const val SUBSCRIBE = "Subscribe with Google Play"
    const val MANAGE = "Manage in Google Play"
    const val PLAY_SUBSCRIPTIONS_URL = "https://play.google.com/store/account/subscriptions"
    const val TERMS_URL = "https://witness.app/legal"

    fun accountTitle(principal: Principal?): String = if (principal == null) NO_ACCOUNT else LINKED_ACCOUNT

    fun accountSubtitle(principal: Principal?): String = if (principal == null) NO_ACCOUNT_HINT else "Linked · ${principal.short}"

    fun precisionLabel(precision: LocationPrecision): String = when (precision) {
        LocationPrecision.HIDDEN -> "Hidden"
        LocationPrecision.AREA -> "City or area"
        LocationPrecision.NEIGHBORHOOD -> "Neighborhood"
        LocationPrecision.EXACT -> "Exact spot"
    }

    fun aiSubtitle(onDevice: Boolean): String = if (onDevice) AI_ON_DEVICE else AI_RULES

    fun monthlyPrice(price: String): String = "$price / month"

    fun perYear(yearly: PlanOffer): String =
        PlanMath.rescale(yearly.price, PlanMath.monthlyEquivalentMicros(yearly.priceMicros))?.let { "per year · $it a month" } ?: "per year"

    fun savings(percent: Int): String? = if (percent > 0) "Save $percent%" else null

    fun eraseNotice(report: EraseReport): Notice {
        val erased = report.erased.size
        val stuck = report.rejected.size + report.missingKey.size
        return when {
            report.offline -> Notice("You're offline. ${copies(erased)} erased so far — try again when connected.", Tone.Warn)
            stuck > 0 -> Notice("${copies(erased)} erased. ${copies(stuck)} need a linked Internet Identity or a takedown request.", Tone.Warn)
            erased == 0 -> Notice("No public copies to erase.", Tone.Muted)
            else -> Notice("${copies(erased)} erased. Proofs stay.", Tone.Ok)
        }
    }

    fun deleteNotice(outcome: DeleteOutcome): Notice = when (outcome) {
        is DeleteOutcome.Deleted -> Notice("Account and hosted media deleted. Anonymous proofs stay.", Tone.Ok)
        is DeleteOutcome.Incomplete -> eraseNotice(outcome.report).let { it.copy(text = "Deletion incomplete. " + it.text) }
    }

    fun exportNotice(outcome: ExportOutcome): Notice = when (outcome) {
        is ExportOutcome.Ready -> Notice("Export ready · ${outcome.records} ${if (outcome.records == 1) "record" else "records"}", Tone.Ok)
        is ExportOutcome.Failed -> Notice("Export failed. Try again.", Tone.Crit)
    }

    fun purchaseNotice(outcome: PurchaseOutcome): Notice? = when (outcome) {
        PurchaseOutcome.Purchased -> Notice("Witness Plus is active.", Tone.Ok)
        PurchaseOutcome.Cancelled -> null
        PurchaseOutcome.NothingToRestore -> Notice("No purchase to restore on this Google account.", Tone.Muted)
        is PurchaseOutcome.Failed -> Notice("Google Play couldn't complete the purchase.", Tone.Crit)
    }

    fun linkError(error: LinkError): String = when (error) {
        LinkError.OFFLINE -> "You're offline. Try again when connected."
        LinkError.REJECTED -> "The registry didn't accept the link. Try again."
        LinkError.NONCE_MISMATCH, LinkError.NO_PENDING_REQUEST -> "That sign-in didn't come from this request. Start again."
        LinkError.PROVIDER_ERROR -> "Internet Identity couldn't sign you in."
        LinkError.MALFORMED, LinkError.UNSUPPORTED_VERSION, LinkError.INVALID_PRINCIPAL -> "The sign-in response wasn't valid. Start again."
    }

    private fun copies(n: Int): String = if (n == 1) "1 public copy" else "$n public copies"
}
