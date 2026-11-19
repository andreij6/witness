package app.witness.feature.proof

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.usecase.EraseMine
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.KeyInfo
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.RecordId
import app.witness.core.ui.CustodyStep
import app.witness.core.ui.Formatters
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class ItemDependencies(
    val captures: CaptureStore,
    val cases: CaseStore,
    val registry: RegistryGateway,
    val erase: EraseMine,
    val links: Links = Links(),
    val time: TimeText = TimeText.UTC,
)

sealed interface EraseState {
    data object Idle : EraseState
    data object Confirm : EraseState
    data object Working : EraseState
    data class Finished(val report: EraseReport) : EraseState
}

@Immutable
data class CaseOption(val id: LocalCaseId?, val title: String, val current: Boolean)

sealed interface MoveState {
    data object Idle : MoveState
    data class Picking(val options: ImmutableList<CaseOption>) : MoveState
    data class Confirm(val target: CaseOption, val from: String?) : MoveState
    data object Working : MoveState
    data object Failed : MoveState
}

sealed interface DeleteState {
    data object Idle : DeleteState
    data class Confirm(val anchored: Boolean) : DeleteState
    data object Working : DeleteState
    data object Failed : DeleteState
    data object Deleted : DeleteState
}

@Immutable
data class LocationProofUi(val coordinates: String, val latE7: Int, val lonE7: Int, val accuracyM: Int, val file: String)

@Immutable
data class ItemUiState(
    val loading: Boolean = true,
    val missing: Boolean = false,
    val recordId: RecordId? = null,
    val image: ByteString? = null,
    val position: String? = null,
    val caseTitle: String? = null,
    val caseSize: Int = 0,
    val capturedTime: String? = null,
    val header: ItemHeader? = null,
    val custody: ImmutableList<CustodyStep> = persistentListOf(),
    val technical: ImmutableList<TechField> = persistentListOf(),
    val certified: Boolean = false,
    val json: String = "{}",
    val webUrl: String? = null,
    val location: LocationProofUi? = null,
    val erase: EraseState = EraseState.Idle,
    val move: MoveState = MoveState.Idle,
    val delete: DeleteState = DeleteState.Idle,
    val anchored: Boolean = false,
) {
    val itemTitle: String get() = position ?: "Photo"
    val itemSubtitle: String get() = Formatters.joinDot(caseTitle, capturedTime?.let { "captured $it" })
    val moveSubtitle: String get() = caseTitle?.let { "Now in $it · ${Formatters.plural(caseSize, "item")}" } ?: "Not in a case"
}

class ItemViewModel(private val recordId: RecordId, private val deps: ItemDependencies) : ViewModel() {
    private val mutable = MutableStateFlow(ItemUiState(recordId = recordId))
    val state: StateFlow<ItemUiState> = mutable.asStateFlow()

    init {
        viewModelScope.launch { load() }
    }

    suspend fun load() {
        val capture = deps.captures.get(recordId)
        if (capture == null) {
            mutable.update { it.copy(loading = false, missing = true) }
            return
        }
        val (image, opening, members) = coroutineScope {
            val image = async { deps.captures.readImage(recordId) }
            val opening = async { deps.captures.opening(recordId) }
            val members = async { capture.caseId?.let { id -> deps.captures.observeAll().first().filter { it.caseId == id } }.orEmpty() }
            Triple(image.await(), opening.await(), members.await())
        }
        val case = capture.caseId?.let { deps.cases.get(it) }
        val (record, key) = remote(capture)
        mutable.update { applyLoaded(it, capture, image, opening, members, case?.title, record, key) }
    }

    private suspend fun remote(capture: Capture): Pair<AnchoredRecord?, KeyInfo?> {
        if (!capture.isVerified) return null to null
        return try {
            val record = deps.registry.get(recordId)
            val keyId = record?.claim?.keyRef?.keyId ?: capture.keyRef?.keyId
            record to keyId?.let { deps.registry.getKey(it) }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null to null
        }
    }

    private fun applyLoaded(
        s: ItemUiState,
        capture: Capture,
        image: ByteString?,
        opening: LocationOpening?,
        members: List<Capture>,
        caseTitle: String?,
        record: AnchoredRecord?,
        key: KeyInfo?,
    ): ItemUiState {
        val time = deps.time
        val position = positionOf(capture, members)
        val technical = ItemFacts.technical(capture, record, key)
        return s.copy(
            loading = false,
            image = image,
            position = position?.let { (i, n) -> "Photo $i of $n" },
            caseTitle = caseTitle,
            caseSize = members.size,
            capturedTime = time.time(capture.createdAtMs),
            header = ItemFacts.header(capture),
            custody = ItemFacts.custody(capture, record, key, opening, time).toImmutableList(),
            technical = technical.toImmutableList(),
            certified = record != null,
            json = ItemFacts.json(technical),
            webUrl = if (capture.isVerified) deps.links.proof(recordId) else null,
            anchored = capture.status == AnchoringStatus.ANCHORED,
            location = opening?.takeIf { it.inRange }?.let {
                LocationProofUi(ItemFacts.coordinates(it), it.latE7, it.lonE7, it.accuracyCm / 100, ItemFacts.locationProof(capture, it, record))
            },
        )
    }

    fun requestErase() = mutable.update { if (it.erase == EraseState.Working) it else it.copy(erase = EraseState.Confirm) }

    fun dismissErase() = mutable.update { if (it.erase == EraseState.Working) it else it.copy(erase = EraseState.Idle) }

    fun confirmErase() {
        if (state.value.erase == EraseState.Working) return
        mutable.update { it.copy(erase = EraseState.Working) }
        viewModelScope.launch {
            val report = try {
                deps.erase.publicCopiesOf(recordId)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                EraseReport(offline = true)
            }
            mutable.update { it.copy(erase = EraseState.Finished(report)) }
        }
    }

    fun requestMove() {
        if (state.value.move != MoveState.Idle) return
        viewModelScope.launch {
            val current = deps.captures.get(recordId)?.caseId
            val options = try {
                deps.cases.observeAll().first().map { CaseOption(it.id, it.title, it.id == current) }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                mutable.update { it.copy(move = MoveState.Failed) }
                return@launch
            }
            val none = CaseOption(null, NO_CASE, current == null)
            mutable.update { it.copy(move = MoveState.Picking((options + none).toImmutableList())) }
        }
    }

    fun pickCase(option: CaseOption) = mutable.update { s ->
        when {
            s.move !is MoveState.Picking -> s
            option.current -> s.copy(move = MoveState.Idle)
            else -> s.copy(move = MoveState.Confirm(option, s.caseTitle))
        }
    }

    fun dismissMove() = mutable.update { if (it.move == MoveState.Working) it else it.copy(move = MoveState.Idle) }

    fun confirmMove() {
        val confirm = state.value.move as? MoveState.Confirm ?: return
        mutable.update { it.copy(move = MoveState.Working) }
        viewModelScope.launch {
            val moved = try {
                val capture = deps.captures.get(recordId)
                if (capture != null) deps.captures.update(capture.copy(caseId = confirm.target.id))
                capture != null
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                false
            }
            if (moved) {
                load()
                mutable.update { it.copy(move = MoveState.Idle) }
            } else {
                mutable.update { it.copy(move = MoveState.Failed) }
            }
        }
    }

    fun requestDelete() = mutable.update { s ->
        if (s.delete == DeleteState.Idle && !s.missing && !s.loading) s.copy(delete = DeleteState.Confirm(s.anchored)) else s
    }

    fun dismissDelete() = mutable.update { if (it.delete is DeleteState.Confirm || it.delete == DeleteState.Failed) it.copy(delete = DeleteState.Idle) else it }

    fun confirmDelete() {
        if (state.value.delete !is DeleteState.Confirm) return
        mutable.update { it.copy(delete = DeleteState.Working) }
        viewModelScope.launch {
            val next = try {
                deps.captures.delete(recordId)
                DeleteState.Deleted
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                DeleteState.Failed
            }
            mutable.update { it.copy(delete = next) }
        }
    }

    companion object {
        const val NO_CASE: String = "Not in a case"

        fun moveTitle(confirm: MoveState.Confirm): String =
            if (confirm.target.id == null) "Remove from ${confirm.from ?: "this case"}?" else "Move to ${confirm.target.title}?"

        fun moveText(confirm: MoveState.Confirm): String =
            if (confirm.target.id == null) "This photo leaves the case report and future share links. Its seal and proof don't change."
            else "This photo joins ${confirm.target.title}'s report and future share links. Its seal and proof don't change."

        fun deleteText(anchored: Boolean): String =
            if (anchored) "The photo and its private details leave this phone. The anchored proof stays — it holds no image, so keep a copy if you'll need to show it."
            else "This photo hasn't been anchored yet. Deleting it now removes it for good and it will never get a public proof."

        fun positionOf(capture: Capture, members: List<Capture>): Pair<Int, Int>? {
            if (capture.caseId == null || members.isEmpty()) return null
            var before = 0
            var found = false
            for (m in members) {
                if (m.recordId == capture.recordId) found = true
                else if (m.createdAtMs < capture.createdAtMs || (m.createdAtMs == capture.createdAtMs && m.recordId < capture.recordId)) before++
            }
            return if (found) (before + 1) to members.size else null
        }

        fun eraseMessage(report: EraseReport): String = when {
            report.offline -> "You're offline. Nothing was erased — try again when you're connected."
            report.complete && report.erased.isEmpty() -> "There were no public copies to erase."
            report.complete -> "Erased ${Formatters.plural(report.erased.size, "public copy", "public copies")}. Their links now show “Removed”."
            else -> "Erased ${report.erased.size}; ${report.rejected.size + report.missingKey.size} couldn't be erased from this phone."
        }
    }
}
