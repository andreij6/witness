package app.witness.feature.proof

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.model.SecurityLevel
import app.witness.core.model.VerdictFlag
import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.CheckResult
import app.witness.core.model.Edit
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import app.witness.core.model.SummarySource
import app.witness.core.model.Verdict
import app.witness.core.ui.Formatters
import app.witness.core.ui.state
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit

class OpenedItem(val label: String, val jpeg: ByteString, val recordId: RecordId? = null)

class OpenedCase(
    val title: String,
    val summary: String?,
    val sharedAtMs: Long,
    val expiresAtMs: Long?,
    val items: List<OpenedItem>,
    val summarySource: SummarySource? = null,
)

object SenderSummaryCopy {
    fun label(source: SummarySource?): String = when (source) {
        SummarySource.GENERATED -> "Sender's summary · AI-drafted"
        SummarySource.EDITED -> "Sender's summary · edited by the sender"
        SummarySource.WRITTEN -> "Sender's summary · written by the sender"
        null -> "Sender's summary"
    }

    fun footnote(source: SummarySource?): String = when (source) {
        SummarySource.GENERATED -> "Drafted by the sender's on-device model · not part of the proof"
        SummarySource.EDITED -> "Drafted on-device, then edited by the sender · not part of the proof"
        SummarySource.WRITTEN -> "Written by the sender · not part of the proof"
        null -> "From the sender · not part of the proof"
    }

    fun isGenerated(source: SummarySource?): Boolean = source == SummarySource.GENERATED
}

sealed interface CaseLinkResult {
    class Opened(val case: OpenedCase) : CaseLinkResult
    data class Expired(val expiredAtMs: Long?) : CaseLinkResult
    data object NotFound : CaseLinkResult
    data object Tampered : CaseLinkResult
}

fun interface CaseLinkOpener {
    suspend fun open(caseId: CaseId, fragment: String): CaseLinkResult
}

@Immutable
data class CaseItemUi(
    val label: String,
    val image: ByteString,
    val meta: String,
    val state: WitnessState?,
    val stateText: String?,
    val recordId: RecordId?,
)

@Immutable
data class CaseVerdictUi(val state: WitnessState, val headline: String, val detail: String)

sealed interface CaseLinkState {
    data object Opening : CaseLinkState
    data class Open(
        val title: String,
        val meta: String,
        val expires: String?,
        val summary: String?,
        val items: ImmutableList<CaseItemUi>,
        val verdict: CaseVerdictUi?,
        val dev: Boolean = false,
        val summarySource: SummarySource? = null,
    ) : CaseLinkState
    data class Expired(val on: String?) : CaseLinkState
    data object NotFound : CaseLinkState
    data object Tampered : CaseLinkState
    data object MissingKey : CaseLinkState
    data object Offline : CaseLinkState
}

class CaseLinkViewModel(
    private val caseText: String,
    private val fragment: String,
    private val opener: CaseLinkOpener,
    private val checkImage: CheckImage,
    private val time: TimeText = TimeText.UTC,
    private val parallelism: Int = PARALLEL_CHECKS,
) : ViewModel() {
    private val mutable = MutableStateFlow<CaseLinkState>(CaseLinkState.Opening)
    val state: StateFlow<CaseLinkState> = mutable.asStateFlow()

    init {
        viewModelScope.launch { open() }
    }

    suspend fun open() {
        val id = CaseId.parseOrNull(caseText.replace("-", "")) ?: return run { mutable.value = CaseLinkState.NotFound }
        if (fragment.isBlank()) return run { mutable.value = CaseLinkState.MissingKey }
        val result = try {
            opener.open(id, fragment)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            return run { mutable.value = CaseLinkState.Offline }
        }
        when (result) {
            is CaseLinkResult.Expired -> mutable.value = CaseLinkState.Expired(result.expiredAtMs?.let(time::date))
            CaseLinkResult.NotFound -> mutable.value = CaseLinkState.NotFound
            CaseLinkResult.Tampered -> mutable.value = CaseLinkState.Tampered
            is CaseLinkResult.Opened -> verify(result.case)
        }
    }

    private suspend fun verify(case: OpenedCase) {
        val items = case.items.mapIndexed { i, item ->
            CaseItemUi(itemLabel(i, item.label), item.jpeg, "Checking…", null, null, item.recordId)
        }.toPersistentList()
        mutable.value = CaseLinkState.Open(
            title = case.title,
            meta = Formatters.joinDot("Shared ${time.date(case.sharedAtMs)}", Formatters.plural(items.size, "item"), case.expiresAtMs?.let { "expires ${time.date(it)}" }),
            expires = case.expiresAtMs?.let(time::dateYear),
            summary = case.summary,
            items = items,
            verdict = null,
            summarySource = case.summarySource.takeIf { case.summary != null },
        )
        val results = arrayOfNulls<CheckResult>(case.items.size)
        val gate = Semaphore(parallelism.coerceAtLeast(1))
        kotlinx.coroutines.coroutineScope {
            case.items.forEachIndexed { i, item ->
                launch {
                    val result = gate.withPermit { judge(item.jpeg) }
                    results[i] = result
                    mutable.update { s ->
                        if (s !is CaseLinkState.Open) s
                        else s.copy(items = s.items.toPersistentList().replacingAt(i, itemUi(i, item, result, time)))
                    }
                }
            }
        }
        val verdict = summarize(results.map { it ?: CheckResult.NO_RECORD })
        val dev = results.any { it != null && isDev(it) }
        mutable.update { s -> if (s is CaseLinkState.Open) s.copy(verdict = verdict, dev = dev) else s }
    }

    private suspend fun judge(jpeg: ByteString): CheckResult =
        try {
            checkImage(jpeg).first { it.isDone }.result ?: CheckResult.NO_RECORD
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            CheckResult.NO_RECORD
        }

    companion object {
        const val PARALLEL_CHECKS: Int = 4

        fun isDev(result: CheckResult): Boolean =
            VerdictFlag.DEV_ATTESTATION in result.flags || result.reference?.claim?.checks?.keySecurity == SecurityLevel.DEV

        fun itemLabel(index: Int, label: String): String = Formatters.joinDot("Photo ${index + 1}", label.takeIf { it.isNotBlank() })

        fun itemUi(index: Int, item: OpenedItem, result: CheckResult, time: TimeText): CaseItemUi {
            val claim = result.reference?.claim
            val blurs = result.declared.filterIsInstance<Edit.Blur>()
            val detail = when {
                blurs.isNotEmpty() -> blurPhrase(blurs)
                claim != null -> "location ${confidenceWord(claim.location.confidence)}"
                else -> null
            }
            val edited = result.verdict == Verdict.MATCHES_WITH_CHANGES && blurs.isNotEmpty()
            val state = if (edited) WitnessState.Edited else result.verdict.state()
            return CaseItemUi(
                label = itemLabel(index, item.label),
                image = item.jpeg,
                meta = Formatters.joinDot(claim?.let { time.timeSeconds(it.time.wallMs) }, detail ?: state.label),
                state = state,
                stateText = if (edited) blurPhrase(blurs).replaceFirstChar { it.uppercase() } else state.shortLabel,
                recordId = result.reference?.recordId ?: item.recordId,
            )
        }

        fun summarize(results: List<CheckResult>): CaseVerdictUi {
            val total = results.size
            var original = 0
            var edited = 0
            var altered = 0
            val blurs = ArrayList<Edit.Blur>()
            for (r in results) when (r.verdict) {
                Verdict.ORIGINAL -> original++
                Verdict.MATCHES_WITH_CHANGES -> {
                    edited++
                    r.declared.filterIsInstanceTo(blurs)
                }
                Verdict.ALTERED -> altered++
                else -> Unit
            }
            val verified = original + edited
            val parts = ArrayList<String>(4)
            if (original > 0) parts += "$original original"
            if (edited > 0) parts += "$edited with ${if (edited == 1) "a declared edit" else "declared edits"}" + (if (blurs.isNotEmpty()) " (${blurPhrase(blurs)})" else "")
            if (altered > 0) parts += "$altered altered"
            val unproven = total - verified - altered
            if (unproven > 0) parts += "$unproven without a record"
            val state = when {
                altered > 0 -> WitnessState.Altered
                verified == total && total > 0 -> WitnessState.Original
                else -> WitnessState.NoRecord
            }
            return CaseVerdictUi(state, Formatters.verifiedSummary(verified, total), parts.joinToString(Formatters.Dot) + ".")
        }

        fun blurPhrase(blurs: List<Edit.Blur>): String {
            val classes = blurs.map { it.blurClass }.distinct().sortedBy(BlurClass::ordinal)
            val nouns = classes.map(ItemFacts::blurNoun)
            val subject = if (nouns.size == 1) nouns[0] else nouns.dropLast(1).joinToString(", ") + " and " + nouns.last()
            return "$subject blurred"
        }

        fun confidenceWord(c: LocationConfidence): String = when (c) {
            LocationConfidence.HIGH -> "high"
            LocationConfidence.MEDIUM -> "medium"
            LocationConfidence.LOW -> "low"
        }
    }
}
