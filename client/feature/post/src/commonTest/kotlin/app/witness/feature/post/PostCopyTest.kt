package app.witness.feature.post

import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PostCopyTest {
    private val hour = 3_600_000L
    private val t = 16 * hour + 12 * 60_000 + 9_000

    private fun capture(status: AnchoringStatus, createdAt: Long = t, anchor: Anchor? = null) = Capture(
        RecordId.fromBytes(ByteArray(10) { it.toByte() }), null, CaptureOrigin.ORIGINAL, status, createdAt, null, LocationConfidence.HIGH, Dims(10, 10), anchor = anchor,
    )

    @Test
    fun clockFormats() {
        assertEquals("4:12:09 PM", PostCopy.clock(t))
        assertEquals("12:00:00 AM", PostCopy.clock(0))
        assertEquals("12:00:00 PM", PostCopy.clock(12 * hour))
        assertEquals("11:59:59 PM", PostCopy.clock(-1_000))
        assertEquals("10:05:00 AM", PostCopy.clock(10 * hour + 5 * 60_000))
    }

    @Test
    fun clockAppliesOffset() = assertEquals("9:12:09 AM", PostCopy.clock(t, -7 * hour))

    @Test
    fun statusPhrases() {
        assertEquals(
            listOf("anchors after setup", "anchoring…", "anchored", "unverified", "anchoring will retry"),
            AnchoringStatus.entries.map(PostCopy::statusPhrase),
        )
    }

    @Test
    fun composerSubtitleMatchesDesign() = assertEquals("Sealed 4:12:09 PM · anchoring…", PostCopy.composerSubtitle(capture(AnchoringStatus.QUEUED), 0))

    @Test
    fun sharedSubtitleAnchored() {
        val c = capture(AnchoringStatus.ANCHORED, anchor = Anchor(1, t + 32_000))
        assertEquals("Sealed 4:12:09 PM · anchored 4:12:41 PM", PostCopy.sharedSubtitle(t, c, 0))
    }

    @Test
    fun sharedSubtitlePending() {
        assertEquals("Sealed 4:12:09 PM · anchoring…", PostCopy.sharedSubtitle(t, capture(AnchoringStatus.QUEUED), 0))
        assertEquals("Sealed 4:12:09 PM · anchoring…", PostCopy.sharedSubtitle(t, capture(AnchoringStatus.QUEUED, anchor = Anchor(1, t)), 0))
    }

    @Test
    fun blurNames() = assertEquals(listOf("Face", "Plate", "House number", "Area"), BlurClass.entries.map(PostCopy::blurName))

    @Test
    fun declaredBlurs() {
        assertEquals("2 declared blurs", PostCopy.declaredBlurs(2))
        assertEquals("1 declared blur", PostCopy.declaredBlurs(1))
    }

    @Test
    fun locationTitles() {
        assertEquals("Location: Denver area", PostCopy.locationTitle(LocationPrecision.AREA, "Denver area", "9xj64"))
        assertEquals("Location: city or area", PostCopy.locationTitle(LocationPrecision.AREA, " ", "9xj64"))
        assertEquals("Location: city or area", PostCopy.locationTitle(LocationPrecision.AREA, null, "9xj64"))
        assertEquals("Location: neighborhood", PostCopy.locationTitle(LocationPrecision.NEIGHBORHOOD, "Denver", "9xj64q"))
        assertEquals("Location: exact spot", PostCopy.locationTitle(LocationPrecision.EXACT, null, "9xj64qrst"))
        assertEquals("Location: hidden", PostCopy.locationTitle(LocationPrecision.HIDDEN, "Denver", null))
        assertEquals("Location: not recorded", PostCopy.locationTitle(LocationPrecision.AREA, "Denver", null))
    }

    @Test
    fun locationSubtitlesAndTones() {
        assertEquals("Precise location hidden, still sealed", PostCopy.locationSubtitle(LocationPrecision.AREA))
        assertEquals("Precise location hidden, still sealed", PostCopy.locationSubtitle(LocationPrecision.HIDDEN))
        assertEquals("Narrower area, easier to pin down", PostCopy.locationSubtitle(LocationPrecision.NEIGHBORHOOD))
        assertEquals("Shows where you were standing", PostCopy.locationSubtitle(LocationPrecision.EXACT))
        assertNull(PostCopy.precisionTone(LocationPrecision.AREA))
        assertEquals(Tone.Warn, PostCopy.precisionTone(LocationPrecision.NEIGHBORHOOD))
        assertEquals(Tone.Crit, PostCopy.precisionTone(LocationPrecision.EXACT))
    }

    @Test
    fun precisionOptionsMatchDesign() {
        val o = PostCopy.precisionOptions("Denver area")
        assertEquals(LocationPrecision.entries, o.map { it.precision })
        assertEquals("City or area · Denver area", o[1].title)
        assertEquals(WitnessIcon.Info, o[2].icon)
        assertEquals(WitnessIcon.Warning, o[3].icon)
        assertEquals("No location on the post", o[0].subtitle)
    }

    @Test
    fun precisionOptionsWithoutPlace() {
        assertEquals("City or area", PostCopy.precisionOptions(null)[1].title)
        assertEquals("City or area", PostCopy.precisionOptions("   ")[1].title)
    }

    @Test
    fun altTitles() {
        assertEquals("Alt text added", PostCopy.altTitle(true))
        assertEquals("Alt text", PostCopy.altTitle(false))
    }

    @Test
    fun linkDisplayStripsScheme() {
        assertEquals("witness.app/p/ABC", PostCopy.linkDisplay("https://witness.app/p/ABC"))
        assertEquals("witness.app/p/ABC", PostCopy.linkDisplay("witness.app/p/ABC"))
        assertEquals("witness.app/p/…", PostCopy.pendingLink("https://witness.app/p/"))
    }

    @Test
    fun shareText() {
        assertEquals("Hail!\nhttps://w/p/X", PostCopy.shareText(" Hail! ", "https://w/p/X", true))
        assertEquals("Hail!", PostCopy.shareText("Hail!", "https://w/p/X", false))
        assertEquals("https://w/p/X", PostCopy.shareText("  ", "https://w/p/X", true))
        assertEquals("", PostCopy.shareText("", "https://w/p/X", false))
    }

    @Test
    fun shows() {
        assertEquals("No location", PostCopy.shows(LocationPrecision.HIDDEN, "Denver"))
        assertEquals("Denver area", PostCopy.shows(LocationPrecision.AREA, "Denver area"))
        assertEquals("City or area", PostCopy.shows(LocationPrecision.AREA, null))
        assertEquals("Neighborhood", PostCopy.shows(LocationPrecision.NEIGHBORHOOD, "Denver"))
        assertEquals("Exact spot", PostCopy.shows(LocationPrecision.EXACT, null))
    }

    @Test
    fun sharedAgoBoundaries() {
        assertEquals("just now", PostCopy.sharedAgo(t, t))
        assertEquals("just now", PostCopy.sharedAgo(t + 59_999, t))
        assertEquals("1 min ago", PostCopy.sharedAgo(t + 60_000, t))
        assertEquals("59 min ago", PostCopy.sharedAgo(t + hour - 1, t))
        assertEquals("1 h ago", PostCopy.sharedAgo(t + hour, t))
        assertEquals("1 day ago", PostCopy.sharedAgo(t + 24 * hour, t))
        assertEquals("3 days ago", PostCopy.sharedAgo(t + 72 * hour, t))
    }

    @Test
    fun sharedAgoClockSkewIsJustNow() = assertEquals("just now", PostCopy.sharedAgo(t - 10_000, t))
}
