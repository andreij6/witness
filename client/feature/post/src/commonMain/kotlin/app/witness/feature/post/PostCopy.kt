package app.witness.feature.post

import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.Capture
import app.witness.core.model.LocationPrecision
import app.witness.core.ui.Formatters

@Immutable
data class PrecisionOption(val precision: LocationPrecision, val title: String, val subtitle: String, val tone: Tone?, val icon: WitnessIcon?)

object PostCopy {
    const val TITLE: String = "New post"
    const val BEFORE_YOU_POST: String = "Before you post"
    const val BLURS_TITLE: String = "Blurs · declared edits"
    const val NO_BLURS: String = "Nothing to blur was found"
    const val PROOF_LINK_TOGGLE: String = "Add proof link to caption"
    const val PROOF_LINK_NOTE: String = "Anyone can check this post, even as a screenshot."
    const val SHEET_TITLE: String = "Location on this post"
    const val SHEET_SUBTITLE: String = "Only changes what people see on the post."
    const val SHEET_NOTE: String = "The precise location stays sealed either way — you can prove it later if you choose."
    const val EDIT_TITLE: String = "Edit photo"
    const val EDIT_NOTE: String = "Every edit is declared in the proof"
    const val BLUR_TOOL_NOTE: String = "Tap the photo to add a blur, tap a blur to remove it"
    const val SHARED_TITLE: String = "Your post has a proof link"
    const val SHARED_NOTE: String = "Anyone can check this post — even a screenshot of it."
    const val PUBLISH_FAILED: String = "Couldn't seal this post. Try again."
    const val LOAD_FAILED: String = "This photo can't be posted"

    fun clock(epochMs: Long, offsetMs: Long = 0L): String {
        val seconds = ((epochMs + offsetMs).mod(DAY_MS) / 1000).toInt()
        val h24 = seconds / 3600
        val h12 = if (h24 % 12 == 0) 12 else h24 % 12
        return buildString(11) {
            append(h12).append(':')
            pad(seconds / 60 % 60).append(':')
            pad(seconds % 60).append(if (h24 < 12) " AM" else " PM")
        }
    }

    private fun StringBuilder.pad(v: Int): StringBuilder = append(if (v < 10) "0" else "").append(v)

    fun statusPhrase(status: AnchoringStatus): String = when (status) {
        AnchoringStatus.SEALED -> "anchors after setup"
        AnchoringStatus.QUEUED -> "anchoring…"
        AnchoringStatus.ANCHORED -> "anchored"
        AnchoringStatus.FAILED -> "anchoring will retry"
        AnchoringStatus.UNVERIFIED -> "unverified"
    }

    fun composerSubtitle(capture: Capture, offsetMs: Long): String =
        "Sealed ${clock(capture.createdAtMs, offsetMs)} · ${statusPhrase(capture.status)}"

    fun sharedSubtitle(sealedAtMs: Long, post: Capture, offsetMs: Long): String {
        val anchored = post.anchor?.anchoredAtMs?.takeIf { post.status == AnchoringStatus.ANCHORED }
        val tail = if (anchored != null) "anchored ${clock(anchored, offsetMs)}" else statusPhrase(post.status)
        return "Sealed ${clock(sealedAtMs, offsetMs)} · $tail"
    }

    fun blurName(c: BlurClass): String = when (c) {
        BlurClass.FACE -> "Face"
        BlurClass.PLATE -> "Plate"
        BlurClass.HOUSE_NO -> "House number"
        BlurClass.OTHER -> "Area"
    }

    fun declaredBlurs(count: Int): String = Formatters.declaredEdits(count, "blur")

    fun locationTitle(precision: LocationPrecision, place: String?, coarse: String?): String = when {
        precision == LocationPrecision.HIDDEN -> "Location: hidden"
        coarse == null -> "Location: not recorded"
        precision == LocationPrecision.AREA -> "Location: ${place?.takeIf { it.isNotBlank() } ?: "city or area"}"
        precision == LocationPrecision.NEIGHBORHOOD -> "Location: neighborhood"
        else -> "Location: exact spot"
    }

    fun locationSubtitle(precision: LocationPrecision): String = when (precision) {
        LocationPrecision.HIDDEN, LocationPrecision.AREA -> "Precise location hidden, still sealed"
        LocationPrecision.NEIGHBORHOOD -> "Narrower area, easier to pin down"
        LocationPrecision.EXACT -> "Shows where you were standing"
    }

    fun precisionTone(precision: LocationPrecision): Tone? = when (precision) {
        LocationPrecision.NEIGHBORHOOD -> Tone.Warn
        LocationPrecision.EXACT -> Tone.Crit
        else -> null
    }

    fun precisionOptions(place: String?): List<PrecisionOption> {
        val area = place?.trim()?.takeIf { it.isNotEmpty() }
        return listOf(
            PrecisionOption(LocationPrecision.HIDDEN, "Hidden", "No location on the post", null, null),
            PrecisionOption(LocationPrecision.AREA, if (area == null) "City or area" else "City or area · $area", "A broad area, not your street", null, null),
            PrecisionOption(LocationPrecision.NEIGHBORHOOD, "Neighborhood", "Narrower area, easier to pin down", Tone.Warn, WitnessIcon.Info),
            PrecisionOption(LocationPrecision.EXACT, "Exact spot", "Shows where you were standing", Tone.Crit, WitnessIcon.Warning),
        )
    }

    fun altTitle(aiGenerated: Boolean): String = if (aiGenerated) "Alt text added" else "Alt text"

    fun linkDisplay(url: String): String = url.substringAfter("://")

    fun pendingLink(proofBase: String): String = linkDisplay(proofBase) + "…"

    fun shareText(caption: String, url: String, includeLink: Boolean): String {
        val c = caption.trim()
        return when {
            !includeLink -> c
            c.isEmpty() -> url
            else -> "$c\n$url"
        }
    }

    fun shows(precision: LocationPrecision, place: String?): String = when (precision) {
        LocationPrecision.HIDDEN -> "No location"
        LocationPrecision.AREA -> place?.trim()?.takeIf { it.isNotEmpty() } ?: "City or area"
        LocationPrecision.NEIGHBORHOOD -> "Neighborhood"
        LocationPrecision.EXACT -> "Exact spot"
    }

    fun sharedAgo(nowMs: Long, sharedAtMs: Long): String {
        val s = (nowMs - sharedAtMs).coerceAtLeast(0) / 1000
        return when {
            s < 60 -> "just now"
            s < 3_600 -> "${s / 60} min ago"
            s < 86_400 -> "${s / 3_600} h ago"
            else -> Formatters.plural(s / 86_400, "day") + " ago"
        }
    }

    private const val DAY_MS: Long = 86_400_000L
}
