package app.witness.feature.post

import androidx.compose.runtime.Immutable
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Post
import app.witness.core.model.PostDraft

sealed interface ComposerSheet {
    data class Location(val selected: LocationPrecision) : ComposerSheet

    data class AltText(val text: String) : ComposerSheet
}

@Immutable
data class ComposerUiState(
    val loading: Boolean = true,
    val subtitle: String = "",
    val draft: PostDraft? = null,
    val place: String? = null,
    val photo: Photo? = null,
    val captionSuggested: Boolean = false,
    val proofLink: Boolean = true,
    val linkText: String = "",
    val published: Post? = null,
    val busy: Boolean = false,
    val sheet: ComposerSheet? = null,
    val editor: EditorState? = null,
    val error: String? = null,
) {
    val enabledBlurs: Int get() = draft?.blurs?.count { it.enabled } ?: 0
    val canShare: Boolean get() = draft != null && !busy
}

sealed interface ComposerEffect {
    data class CopyLink(val url: String) : ComposerEffect

    data class Share(val text: String, val post: Post) : ComposerEffect
}

interface ComposerActions {
    fun toggleBlur(index: Int)

    fun openLocation()

    fun selectPrecision(precision: LocationPrecision)

    fun confirmLocation()

    fun openAltText()

    fun editAltText(text: String)

    fun confirmAltText()

    fun dismissSheet()

    fun setCaption(text: String)

    fun setProofLink(enabled: Boolean)

    fun flagAi()

    fun openEditor()

    fun editor(transform: (EditorState) -> EditorState)

    fun closeEditor()

    fun applyEditor()

    fun copyLink()

    fun share()

    fun dismissError()
}
