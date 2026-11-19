package app.witness.feature.cases

import androidx.compose.foundation.layout.BoxScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.CaseSummary
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters

typealias RecordImage = @Composable BoxScope.(RecordId) -> Unit

enum class CaseFilter(val label: String) { ALL("All"), OPEN("Open"), SHARED("Shared") }

@Immutable
data class CaseCardUi(
    val id: LocalCaseId,
    val title: String,
    val meta: String,
    val anchored: Int,
    val queued: Int,
    val covers: List<RecordId>,
    val sharedLabel: String?,
) {
    val shared: Boolean get() = sharedLabel != null
}

@Immutable
data class EvidenceItemUi(val recordId: RecordId, val state: WitnessState)

object CasePresentation {
    fun evidenceState(capture: Capture): WitnessState = when {
        !capture.isVerified -> WitnessState.Unverified
        capture.status == AnchoringStatus.ANCHORED -> WitnessState.Anchored
        capture.status == AnchoringStatus.UNVERIFIED -> WitnessState.Unverified
        else -> WitnessState.Queued
    }

    fun activeShareCount(case: Case, nowMs: Long): Int = case.shares.count { !it.isExpired(nowMs) }

    fun soonestExpiry(case: Case, nowMs: Long): Long? {
        var soonest: Long? = null
        for (s in case.shares) {
            val at = s.expiresAtMs ?: continue
            if (at > nowMs && (soonest == null || at < soonest)) soonest = at
        }
        return soonest
    }

    fun sharedLabel(activeLinks: Int): String? =
        if (activeLinks <= 0) null else "Shared · " + Formatters.plural(activeLinks, "link")

    fun itemCount(count: Int): String = Formatters.plural(count, "item")

    fun places(captures: List<Capture>): Map<LocalCaseId, String> {
        val best = HashMap<LocalCaseId, Capture>()
        for (c in captures) {
            val id = c.caseId ?: continue
            if (c.place.isNullOrBlank()) continue
            val current = best[id]
            if (current == null || c.createdAtMs > current.createdAtMs) best[id] = c
        }
        return best.mapValues { it.value.place!! }
    }

    fun card(summary: CaseSummary, place: String?, nowMs: Long, offset: UtcOffset): CaseCardUi {
        val case = summary.case
        val expiry = soonestExpiry(case, nowMs)
        val meta = Formatters.joinDot(
            CaseDates.day(case.createdAtMs, nowMs, offset),
            place,
            itemCount(summary.itemCount),
            expiry?.let { "Expires " + CaseDates.monthDay(it, nowMs, offset) },
        )
        return CaseCardUi(
            id = case.id,
            title = case.title,
            meta = meta,
            anchored = summary.anchoredCount,
            queued = summary.queuedCount,
            covers = summary.coverRecords,
            sharedLabel = sharedLabel(activeShareCount(case, nowMs)),
        )
    }

    fun filter(cards: List<CaseCardUi>, filter: CaseFilter): List<CaseCardUi> = when (filter) {
        CaseFilter.ALL -> cards
        CaseFilter.OPEN -> cards.filter { !it.shared }
        CaseFilter.SHARED -> cards.filter { it.shared }
    }

    fun caseSubtitle(case: Case, place: String?, nowMs: Long, offset: UtcOffset): String =
        Formatters.joinDot(place, CaseDates.day(case.createdAtMs, nowMs, offset) + " " + CaseDates.time(case.createdAtMs, offset))

    fun optionSubtitle(case: Case, itemCount: Int): String = Formatters.joinDot(ReportType.label(case.templateId), itemCount(itemCount))
}
