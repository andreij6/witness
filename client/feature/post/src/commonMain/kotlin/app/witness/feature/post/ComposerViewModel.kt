package app.witness.feature.post

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.domain.usecase.PostException
import app.witness.core.domain.usecase.PostText
import app.witness.core.model.Capture
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Post
import app.witness.core.model.PostDraft
import app.witness.core.model.RecordId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class ComposerViewModel(
    private val parent: RecordId,
    private val buildPost: BuildPost,
    private val captures: CaptureStore,
    private val links: Links = Links(),
    private val utcOffset: (Long) -> Long = { 0L },
    private val telemetry: Telemetry = Telemetry.None,
) : ViewModel(), ComposerActions {
    private val local = MutableStateFlow(ComposerUiState(linkText = PostCopy.pendingLink(links.proofBase)))
    private val effectChannel = Channel<ComposerEffect>(Channel.BUFFERED)

    val effects: Flow<ComposerEffect> = effectChannel.receiveAsFlow()

    private val parentCapture: Flow<Capture?> = captures.observeAll().map { list -> list.firstOrNull { it.recordId == parent } }.distinctUntilChanged()

    val state: StateFlow<ComposerUiState> = combine(local, parentCapture) { s, c ->
        if (c == null) s else s.copy(subtitle = PostCopy.composerSubtitle(c, utcOffset(c.createdAtMs)), place = c.place)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), local.value)

    init {
        viewModelScope.launch { load() }
    }

    private suspend fun load() {
        try {
            val draft = buildPost.draft(parent)
            val photo = captures.readImage(draft.parent)?.let { Photo(it.toByteArray()) }
            val place = captures.get(draft.parent)?.place
            local.update { it.copy(loading = false, draft = draft, photo = photo, place = place, captionSuggested = draft.caption.isNotEmpty()) }
        } catch (e: CancellationException) {
            throw e
        } catch (e: PostException) {
            local.update { it.copy(loading = false, error = PostCopy.LOAD_FAILED) }
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            local.update { it.copy(loading = false, error = PostCopy.LOAD_FAILED) }
        }
    }

    private inline fun editDraft(sealed: Boolean, crossinline f: (PostDraft) -> PostDraft) = local.update { s ->
        val d = s.draft ?: return@update s
        val next = f(d)
        if (sealed && next != d) s.copy(draft = next, published = null, linkText = PostCopy.pendingLink(links.proofBase)) else s.copy(draft = next)
    }

    override fun toggleBlur(index: Int) = editDraft(sealed = true) { buildPost.toggleBlur(it, index) }

    override fun openLocation() = local.update { s -> s.draft?.let { s.copy(sheet = ComposerSheet.Location(it.precision)) } ?: s }

    override fun selectPrecision(precision: LocationPrecision) = local.update { s ->
        if (s.sheet is ComposerSheet.Location) s.copy(sheet = ComposerSheet.Location(precision)) else s
    }

    override fun confirmLocation() {
        val sheet = local.value.sheet as? ComposerSheet.Location ?: return
        val draft = local.value.draft ?: return
        viewModelScope.launch {
            val next = buildPost.withPrecision(draft, sheet.selected)
            local.update { s -> s.copy(sheet = null) }
            editDraft(sealed = true) { it.copy(precision = next.precision, coarse = next.coarse, locationWarning = next.locationWarning) }
        }
    }

    override fun openAltText() = local.update { s -> s.draft?.let { s.copy(sheet = ComposerSheet.AltText(it.altText)) } ?: s }

    override fun editAltText(text: String) = local.update { s ->
        if (s.sheet is ComposerSheet.AltText) s.copy(sheet = ComposerSheet.AltText(text.take(PostText.MAX_ALT))) else s
    }

    override fun confirmAltText() {
        val sheet = local.value.sheet as? ComposerSheet.AltText ?: return
        local.update { it.copy(sheet = null) }
        editDraft(sealed = false) { d ->
            val clean = PostText.clean(sheet.text, PostText.MAX_ALT) ?: PostText.rulesAltText(d.blurs, local.value.place)
            if (clean == d.altText) d else d.copy(altText = clean, aiGenerated = false)
        }
    }

    override fun dismissSheet() = local.update { it.copy(sheet = null) }

    override fun setCaption(text: String) {
        local.update { it.copy(captionSuggested = false) }
        editDraft(sealed = false) { it.copy(caption = text.take(PostText.MAX_CAPTION)) }
    }

    override fun setProofLink(enabled: Boolean) = local.update { it.copy(proofLink = enabled) }

    override fun flagAi() {
        val suggested = local.value.captionSuggested
        local.update { it.copy(captionSuggested = false) }
        editDraft(sealed = false) { d ->
            d.copy(
                altText = if (d.aiGenerated) PostText.rulesAltText(d.blurs, local.value.place) else d.altText,
                caption = if (suggested) "" else d.caption,
                aiGenerated = false,
            )
        }
        telemetry.breadcrumb("post_ai_flagged")
    }

    override fun openEditor() = local.update { s -> s.draft?.let { s.copy(editor = PhotoEditor.open(it)) } ?: s }

    override fun editor(transform: (EditorState) -> EditorState) = local.update { s -> s.editor?.let { s.copy(editor = transform(it)) } ?: s }

    override fun closeEditor() = local.update { it.copy(editor = null) }

    override fun applyEditor() {
        val editor = local.value.editor ?: return
        local.update { it.copy(editor = null) }
        editDraft(sealed = true) { PhotoEditor.apply(it, editor) }
    }

    override fun copyLink() {
        viewModelScope.launch { publish()?.let { effectChannel.send(ComposerEffect.CopyLink(it.proofUrl)) } }
    }

    override fun share() {
        viewModelScope.launch {
            val post = publish() ?: return@launch
            val s = local.value
            effectChannel.send(ComposerEffect.Share(PostCopy.shareText(s.draft?.caption.orEmpty(), post.proofUrl, s.proofLink), post))
        }
    }

    override fun dismissError() = local.update { it.copy(error = null) }

    private suspend fun publish(): Post? {
        val s = local.value
        s.published?.let { return it }
        val draft = s.draft ?: return null
        if (s.busy) return null
        local.update { it.copy(busy = true, error = null) }
        return try {
            buildPost.publish(draft).also { post ->
                local.update { cur ->
                    if (cur.draft?.blurs == draft.blurs && cur.draft.precision == draft.precision && cur.draft.adjustments == draft.adjustments)
                        cur.copy(published = post, linkText = PostCopy.linkDisplay(post.proofUrl))
                    else cur
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            local.update { it.copy(error = PostCopy.PUBLISH_FAILED) }
            null
        } finally {
            local.update { it.copy(busy = false) }
        }
    }

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
    }
}
