package app.witness.feature.proof

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.usecase.Geohash
import app.witness.core.domain.usecase.ProofFacts
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.Edit
import app.witness.core.model.KeyInfo
import app.witness.core.model.RecordId
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.isPresentationOnly
import app.witness.core.ui.Formatters
import app.witness.core.ui.ProofFact
import app.witness.core.ui.headline
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@Immutable
data class ProofPage(
    val recordId: RecordId,
    val verdict: Verdict,
    val headline: String,
    val qualifier: String?,
    val qualifierNote: String?,
    val badge: String?,
    val summary: String,
    val warning: String?,
    val dev: Boolean = false,
    val facts: ImmutableList<ProofFact>,
    val image: ByteString,
    val aspect: Float?,
)

sealed interface ProofPageState {
    data object Loading : ProofPageState
    data class Found(val page: ProofPage) : ProofPageState
    data class Removed(val anchoredAtMs: Long) : ProofPageState
    data object NotFound : ProofPageState
    data object Offline : ProofPageState
}

class ProofPageViewModel(
    private val recordText: String,
    private val registry: RegistryGateway,
    private val media: MediaGateway,
    private val time: TimeText = TimeText.UTC,
) : ViewModel() {
    private val mutable = MutableStateFlow<ProofPageState>(ProofPageState.Loading)
    val state: StateFlow<ProofPageState> = mutable.asStateFlow()

    init {
        viewModelScope.launch { mutable.value = load() }
    }

    fun retry() {
        mutable.value = ProofPageState.Loading
        viewModelScope.launch { mutable.value = load() }
    }

    suspend fun load(): ProofPageState {
        val id = RecordId.parseOrNull(recordText.replace("-", "")) ?: return ProofPageState.NotFound
        return try {
            coroutineScope<ProofPageState> {
                val image = async { media.reference(id) }
                val record = registry.get(id) ?: return@coroutineScope ProofPageState.NotFound.also { image.cancel() }
                val key = async { registry.getKey(record.claim.keyRef.keyId) }
                val bytes = image.await() ?: return@coroutineScope ProofPageState.Removed(record.anchoredAtMs).also { key.cancel() }
                ProofPageState.Found(page(record, key.await(), bytes, time))
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            ProofPageState.Offline
        }
    }

    companion object {
        fun page(record: AnchoredRecord, key: KeyInfo?, image: ByteString, time: TimeText): ProofPage {
            val claim = record.claim
            val edits = claim.kind.edits
            val original = edits.isPresentationOnly()
            val verdict = if (original) Verdict.ORIGINAL else Verdict.MATCHES_WITH_CHANGES
            val blurs = edits.count { it is Edit.Blur }
            val others = edits.count { it.editClass == app.witness.core.model.EditClass.GLOBAL }
            val qualifier = when {
                original -> null
                blurs > 0 && others == 0 -> "with ${Formatters.declaredEdits(blurs, "blur")}"
                else -> "with ${Formatters.declaredEdits(blurs + others, "edit")}"
            }
            val flags = ProofFacts.flags(claim, Anchor(record.recordIx, record.anchoredAtMs), key)
            val warning = buildList {
                if (VerdictFlag.KEY_REVOKED_AFTER_ANCHOR in flags) add("The phone's key was revoked after this was anchored.")
                if (VerdictFlag.LATE_ANCHOR in flags) add("Anchored more than a day after capture.")
                if (VerdictFlag.CLOCK_MISMATCH in flags) add("The phone's clock disagreed with satellite time.")
                if (VerdictFlag.DEV_ATTESTATION in flags) add("DEV · captured on a development device.")
            }.takeIf { it.isNotEmpty() }?.joinToString(" ")
            val dev = VerdictFlag.DEV_ATTESTATION in flags
            val device = if (dev) "Captured on a development device." else "Captured on a genuine phone."
            val summary = when {
                original -> "$device Unaltered since capture."
                blurs > 0 && others == 0 -> "$device Only the ${Formatters.declaredEdits(blurs, "blur")} differ."
                else -> "$device Only the photographer's declared edits differ."
            }
            return ProofPage(
                recordId = record.recordId,
                verdict = verdict,
                headline = verdict.headline(),
                qualifier = qualifier,
                qualifierNote = if (original) null else "the photographer's own edits",
                badge = if (blurs > 0) Formatters.declaredEdits(blurs, "blur") else if (!original) Formatters.declaredEdits(others, "edit") else null,
                summary = summary,
                warning = warning,
                dev = dev,
                facts = facts(record, key, time).toImmutableList(),
                image = image,
                aspect = claim.dims.takeIf { it.isValid }?.let { it.width.toFloat() / it.height },
            )
        }

        fun facts(record: AnchoredRecord, key: KeyInfo?, time: TimeText): List<ProofFact> {
            val claim = record.claim
            val security = ItemFacts.securityLabel(key?.security ?: claim.checks.keySecurity)
            return listOf(
                ProofFact("Captured", Formatters.joinDot(time.dateYearTime(claim.time.wallMs), if (claim.time.gnssMs != null) "GNSS time" else "phone clock"), WitnessIcon.Schedule),
                ProofFact("Location", Formatters.joinDot(claim.coarse?.let { Geohash.area(it) ?: "Area $it" } ?: "Area hidden", if (claim.location.commit != null) "precise sealed" else null), WitnessIcon.NearMe),
                ProofFact("Device", Formatters.joinDot("Android phone", security, if (key?.isDev == true) "dev attestation" else "attested"), WitnessIcon.Key),
                ProofFact("Anchored", Formatters.joinDot(time.time(record.anchoredAtMs), "Internet Computer"), WitnessIcon.Verified),
            )
        }
    }
}
