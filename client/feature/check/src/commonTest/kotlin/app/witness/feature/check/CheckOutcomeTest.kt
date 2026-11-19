package app.witness.feature.check

import app.witness.core.designsystem.Tone
import app.witness.core.model.BlurClass
import app.witness.core.model.CheckResult
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Rect
import app.witness.core.model.Region
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import app.witness.core.ui.FactStatus
import app.witness.core.ui.NormRect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CheckOutcomeTest {
    private val one = Region.Q16_ONE
    private val time = TimeText.UTC
    private fun blur(c: BlurClass) = Edit.Blur(Rect(0, 0, 10, 10), c, 100)

    @Test
    fun noRecordListsWhatWasChecked() {
        val o = CheckOutcome.of(CheckResult.NO_RECORD, null, time)
        assertEquals("No Witness record", o.headline)
        assertEquals(listOf("Watermark", "Fingerprints", "Credentials"), o.facts.map { it.label })
        assertEquals(listOf("None found", "No match", "No C2PA data in this file"), o.facts.map { it.value })
        assertFalse(o.hasReference)
        assertTrue(o.changes.isEmpty())
        assertNull(o.captured)
    }

    @Test
    fun rejectedProofsAreADistinctStateNotANoRecord() {
        val o = CheckOutcome.of(CheckResult(Verdict.NO_RECORD, proofsRejected = true), null, time)
        assertTrue(o.registryUnverified)
        assertEquals(CheckOutcome.UNVERIFIED_HEADLINE, o.headline)
        assertNull(o.qualifier)
        assertEquals(CheckOutcome.UNVERIFIED_BODY, o.body)
        assertFalse(o.body.contains("found no sealed capture"))
        assertEquals(listOf("Registry", "Watermark", "Fingerprints", "Credentials"), o.facts.map { it.label })
        assertEquals(FactStatus.Failed, o.facts.first().status)
        assertNull(o.note)
        assertFalse(o.hasReference)
        assertFalse(CheckOutcome.of(CheckResult.NO_RECORD, null, time).registryUnverified)
    }

    @Test
    fun rejectedProofsBesideAMatchOnlyAddANote() {
        val o = CheckOutcome.of(Fixtures.result(Verdict.ORIGINAL).copy(proofsRejected = true), null, time)
        assertFalse(o.registryUnverified)
        assertEquals("Original", o.headline)
        assertEquals("Some registry answers failed verification and were ignored.", o.note)
        assertTrue(o.hasReference)
        assertNull(CheckOutcome.of(Fixtures.result(Verdict.ORIGINAL), null, time).note)
    }

    @Test
    fun noRecordReasonsChangeTheChecklist() {
        val r = CheckResult(Verdict.NO_RECORD, reasons = EnumBits.of(VerdictReason.FOREIGN_WATERMARK, VerdictReason.UNCONFIRMED, VerdictReason.OTHER_CREDENTIALS))
        val o = CheckOutcome.of(r, null, time)
        assertEquals(listOf("From a different photo", "Similar photos, none confirmed", "Content Credentials from another app"), o.facts.map { it.value })
        assertTrue(o.note!!.contains("different photo"))
        val u = CheckOutcome.checked(CheckResult(Verdict.NO_RECORD, reasons = EnumBits.of(VerdictReason.UNDECODABLE, VerdictReason.WATERMARK_UNAVAILABLE, VerdictReason.KEY_NOT_TRUSTED)))
        assertEquals(listOf("Reader unavailable here", "Couldn't read this file", "Signed by a key that isn't trusted"), u.map { it.value })
    }

    @Test
    fun noRecordIgnoresAnyReference() {
        val o = CheckOutcome.of(Fixtures.result(Verdict.NO_RECORD), null, time)
        assertNull(o.recordId)
        assertNull(o.matchedBy)
    }

    @Test
    fun matchesWithChangesDescribesCropRecompressAndBlurs() {
        val ref = Fixtures.reference(compared = one * 3 / 4, changed = one / 32)
        val r = Fixtures.result(Verdict.MATCHES_WITH_CHANGES, ref, declared = listOf(blur(BlurClass.HOUSE_NO), blur(BlurClass.PLATE)))
        val o = CheckOutcome.of(r, Dims(1080, 1080), time)
        assertEquals("Matches an original", o.headline)
        assertEquals("with changes", o.qualifier)
        assertEquals(listOf("Cropped to a square", "Recompressed", "2 blurs declared"), o.changes.map { it.title })
        assertEquals("About 25% of the frame removed", o.changes[0].detail)
        assertEquals("97% of the compared area matches", o.changes[1].detail)
        assertEquals("Plate and house number", o.changes[2].detail)
        assertEquals("Declared by sender", o.changes[2].by)
        assertEquals("With 2 declared blurs", o.referenceCaption)
        assertEquals(Fixtures.recordId, o.recordId)
        assertTrue(o.hasReference)
        assertEquals("Fingerprint", o.matchedBy)
        assertEquals("75% of reference frame", o.aligned)
        assertEquals(1080f / 1440f, o.referenceAspect)
        assertEquals(NormRect.centeredCrop(1080f / 1440f, 1f), o.crop)
    }

    @Test
    fun nonSquareCropAndNoCropWhenFullyCompared() {
        val cropped = CheckOutcome.of(Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(compared = one / 2)), Dims(1600, 900), time)
        assertEquals("Cropped", cropped.changes.first().title)
        val full = CheckOutcome.of(Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference()), Dims(1080, 1440), time)
        assertEquals(listOf("Recompressed"), full.changes.map { it.title })
        assertNull(full.crop)
    }

    @Test
    fun cropRectNeedsBothAspects() {
        val o = CheckOutcome.of(Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(compared = one / 2)), null, time)
        assertNull(o.crop)
        assertEquals("Cropped", o.changes.first().title)
    }

    @Test
    fun globalDeclaredEditsAreSummarised() {
        val r = Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(MatchEvidence.SEAL), declared = listOf(Edit.Tone(1, 2, 3), Edit.Filter(1, 50), Edit.Tone(4, 5, 6)))
        val o = CheckOutcome.of(r, null, time)
        val line = o.changes.single()
        assertEquals("Declared edits", line.title)
        assertEquals("Tone, filter", line.detail)
        assertEquals(Tone.Edit, line.tone)
    }

    @Test
    fun couldntCompareAddsLineAndNote() {
        val r = Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(MatchEvidence.GRID)).copy(flags = EnumBits.of(VerdictFlag.COULDNT_COMPARE))
        val o = CheckOutcome.of(r, null, time)
        assertEquals("Some regions couldn't be compared", o.changes.last().title)
        assertEquals("Some regions couldn't be compared.", o.note)
    }

    @Test
    fun noPublicReferenceCapsTheHeadline() {
        val r = Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(MatchEvidence.GRID)).copy(flags = EnumBits.of(VerdictFlag.NO_PUBLIC_REFERENCE))
        val o = CheckOutcome.of(r, null, time)
        assertEquals("Matches a sealed photo", o.headline)
        assertEquals("with no public reference", o.qualifier)
        assertTrue(o.body.contains("never posted"))
    }

    @Test
    fun alteredHighlightsRegionsAndFacts() {
        val regions = listOf(Region(one / 2, one / 2, one / 8, one / 16))
        val o = CheckOutcome.of(Fixtures.result(Verdict.ALTERED, regions = regions), null, time)
        assertEquals("Altered", o.headline)
        assertEquals("Changed in 1 region", o.changes.first().title)
        assertEquals(Tone.Crit, o.changes.first().tone)
        assertEquals(listOf("Content", "Match", "Device", "Anchored"), o.facts.map { it.label })
        assertEquals(FactStatus.Failed, o.facts[0].status)
        assertEquals("Differs in 1 region", o.facts[0].value)
        assertTrue(o.facts.drop(1).all { it.status == FactStatus.Passed })
        assertEquals("Nov 18, 2:13 PM", o.facts.last().value)
        assertEquals(1, o.regions.size)
        assertEquals(0.5f, o.regions[0].left)
    }

    @Test
    fun alteredBodyCountsRegionsOrBrokenSeal() {
        val two = Fixtures.result(Verdict.ALTERED, regions = listOf(Region(0, 0, 10, 10), Region(10, 10, 10, 10)))
        assertTrue(CheckOutcome.body(two, false).contains("2 regions were changed"))
        val broken = CheckResult(Verdict.ALTERED, reasons = EnumBits.of(VerdictReason.SEAL_BROKEN), reference = Fixtures.reference(MatchEvidence.SEAL))
        assertTrue(CheckOutcome.body(broken, false).contains("seal is broken"))
        assertEquals("Seal broken", CheckOutcome.of(broken, null, time).facts.first().value)
    }

    @Test
    fun unanchoredShowsPendingAnchor() {
        val o = CheckOutcome.of(Fixtures.result(Verdict.SEALED_UNANCHORED, Fixtures.reference(MatchEvidence.SEAL)), null, time)
        assertEquals("Sealed · not yet anchored", o.headline)
        assertEquals(listOf("Valid", "Genuine", "Pending"), o.facts.map { it.badge?.text })
        assertEquals(Tone.Warn, o.facts.last().badge?.tone)
        assertTrue(o.changes.isEmpty())
    }

    @Test
    fun originalHasMatchFactsAndNoChanges() {
        val o = CheckOutcome.of(Fixtures.result(Verdict.ORIGINAL, Fixtures.reference(MatchEvidence.EXACT_HASH)), null, time)
        assertTrue(o.changes.isEmpty())
        assertEquals("Exact file", o.facts.first().value)
        assertEquals("Captured Nov 18, 2:12 PM · near 39.7° N, 105.0° W · genuine Android phone", o.captured)
        assertEquals("Nov 18 · 2:12 PM", o.referenceCaption)
    }

    @Test
    fun devAttestedCaptureNeverClaimsAGenuinePhone() {
        val flagged = Fixtures.result(Verdict.ORIGINAL, Fixtures.reference(MatchEvidence.EXACT_HASH)).let { it.copy(flags = EnumBits.of(VerdictFlag.DEV_ATTESTATION)) }
        val o = CheckOutcome.of(flagged, null, time)
        assertEquals("Captured Nov 18, 2:12 PM · near 39.7° N, 105.0° W · development device", o.captured)
        assertTrue(o.captured!!.contains("genuine").not())
    }

    @Test
    fun matchedByCoversEveryEvidence() {
        val plain = CheckResult(Verdict.ORIGINAL)
        val wm = CheckResult(Verdict.ORIGINAL, reasons = EnumBits.of(VerdictReason.WATERMARK_FOUND, VerdictReason.EXACT_PIXEL_HASH))
        assertEquals("Content Credentials", CheckOutcome.matchedBy(MatchEvidence.SEAL, plain))
        assertEquals("Exact file", CheckOutcome.matchedBy(MatchEvidence.EXACT_HASH, plain))
        assertEquals("Exact pixels", CheckOutcome.matchedBy(MatchEvidence.EXACT_HASH, wm))
        assertEquals("Fingerprint", CheckOutcome.matchedBy(MatchEvidence.PIXELS, plain))
        assertEquals("Watermark + fingerprint", CheckOutcome.matchedBy(MatchEvidence.PIXELS, wm))
        assertEquals("Region grid", CheckOutcome.matchedBy(MatchEvidence.GRID, plain))
        assertEquals("Watermark + region grid", CheckOutcome.matchedBy(MatchEvidence.GRID, wm))
    }

    @Test
    fun devDevicesAreCalledOut() {
        val ref = Fixtures.reference(claim = Fixtures.claim(security = SecurityLevel.DEV))
        assertEquals("Development device · not production", CheckOutcome.device(ref, CheckResult(Verdict.ORIGINAL)))
        val flagged = CheckResult(Verdict.ORIGINAL, flags = EnumBits.of(VerdictFlag.DEV_ATTESTATION))
        assertEquals("Development device · not production", CheckOutcome.device(Fixtures.reference(), flagged))
        assertEquals("Genuine Android phone · attested", CheckOutcome.device(Fixtures.reference(), CheckResult(Verdict.ORIGINAL)))
    }

    @Test
    fun notesJoinEveryFlagInOrder() {
        val r = CheckResult(Verdict.ORIGINAL, flags = EnumBits.of(VerdictFlag.LATE_ANCHOR, VerdictFlag.KEY_REVOKED_AFTER_ANCHOR, VerdictFlag.TRUNCATED_CANDIDATES))
        val note = CheckOutcome.note(r)!!
        assertTrue(note.indexOf("revoked") < note.indexOf("more than a day"))
        assertTrue(note.endsWith("complete search."))
        assertNull(CheckOutcome.note(CheckResult(Verdict.ORIGINAL)))
    }

    @Test
    fun blurNounsGroupAndCount() {
        assertEquals("Face", CheckOutcome.blurNouns(listOf(blur(BlurClass.FACE))))
        assertEquals("2 faces and plate", CheckOutcome.blurNouns(listOf(blur(BlurClass.FACE), blur(BlurClass.PLATE), blur(BlurClass.FACE))))
        assertEquals("Face, plate and house number", CheckOutcome.blurNouns(listOf(blur(BlurClass.HOUSE_NO), blur(BlurClass.PLATE), blur(BlurClass.FACE))))
    }

    @Test
    fun percentRoundsAndClamps() {
        assertEquals(0, CheckOutcome.percent(-5))
        assertEquals(100, CheckOutcome.percent(one * 2))
        assertEquals(50, CheckOutcome.percent(one / 2))
        assertEquals(1, CheckOutcome.percent(one / 100))
    }

    @Test
    fun normalisedRegionsStayInsideTheUnitSquare() {
        val r = CheckOutcome.norm(Region(one - 10, one - 10, one, one))
        assertTrue(r.right <= 1f && r.bottom <= 1f && r.left <= r.right)
        val z = CheckOutcome.norm(Region(0, 0, 0, 0))
        assertEquals(0f, z.width)
    }

    @Test
    fun editNounsCoverAllKinds() {
        val edits = listOf(Edit.Resize(1, 1), Edit.Recompress(app.witness.core.model.ImageFormat.JPEG, 80), Edit.Orient(1), Edit.Crop(Rect(0, 0, 1, 1)), Edit.Rotate(1), Edit.Tone(0, 0, 0), Edit.Filter(0, 0), blur(BlurClass.OTHER))
        assertEquals(listOf("resize", "recompress", "orientation", "crop", "straighten", "tone", "filter", "blur"), edits.map(CheckOutcome::editNoun))
    }

    @Test
    fun derivativeClaimKeepsDeterministicOutput() {
        val claim = Fixtures.claim(kind = ClaimKind.Derivative(Fixtures.recordId, Fixtures.claim().captureId, emptyList()))
        val r = Fixtures.result(Verdict.MATCHES_WITH_CHANGES, Fixtures.reference(claim = claim))
        assertEquals(CheckOutcome.of(r, null, time), CheckOutcome.of(r, null, time))
        assertNotNull(CheckOutcome.of(r, null, time).captured)
    }
}
