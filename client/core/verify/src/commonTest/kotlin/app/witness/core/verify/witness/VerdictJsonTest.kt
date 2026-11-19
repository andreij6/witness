package app.witness.core.verify.witness

import app.witness.core.icp.witness.WireException
import app.witness.core.model.Anchor
import app.witness.core.model.BlurClass
import app.witness.core.model.Edit
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Rect
import app.witness.core.model.Region
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class VerdictJsonTest {
    @Test
    fun noRecordVerdictParses() {
        val r = VerdictJson.parse(Verdicts.NO_RECORD)
        assertEquals(Verdict.NO_RECORD, r.verdict)
        assertTrue(r.flags.isEmpty)
        assertTrue(VerdictReason.NO_CANDIDATES in r.reasons && VerdictReason.WATERMARK_UNAVAILABLE in r.reasons)
        assertNull(r.reference)
        assertEquals(1, r.version)
    }

    @Test
    fun matchWithReferenceParsesEveryField() {
        val r = VerdictJson.parse(Verdicts.WITH_CHANGES)
        assertEquals(Verdict.MATCHES_WITH_CHANGES, r.verdict)
        assertTrue(VerdictFlag.DEV_ATTESTATION in r.flags && VerdictFlag.COULDNT_COMPARE in r.flags)
        assertTrue(VerdictReason.EXACT_PIXEL_HASH in r.reasons)
        assertEquals(listOf<Edit>(Edit.Blur(Rect(10, 20, 30, 40), BlurClass.FACE, 800)), r.declared)
        assertEquals(listOf(Region(1, 2, 3, 4)), r.regions)
        val ref = r.reference!!
        assertEquals(Verdicts.RECORD_ID, ref.recordId.text)
        assertEquals(Anchor(9, 1_700_000_001_000), ref.anchor)
        assertEquals(MatchEvidence.EXACT_HASH, ref.evidence)
        assertEquals(65536, ref.confidenceQ16)
        assertEquals(100, ref.changedQ16)
        assertEquals(640, ref.claim.dims.width)
    }

    @Test
    fun everyOutcomeAndEvidenceNameIsKnown() {
        for (v in Verdict.entries) assertEquals(v, VerdictJson.parse(Verdicts.NO_RECORD.replace("NO_RECORD", v.name)).verdict)
        for ((name, e) in listOf("seal" to MatchEvidence.SEAL, "exact_hash" to MatchEvidence.EXACT_HASH, "pixels" to MatchEvidence.PIXELS, "grid" to MatchEvidence.GRID)) {
            assertEquals(e, VerdictJson.parse(Verdicts.WITH_CHANGES.replace("\"exact_hash\"", "\"$name\"")).reference?.evidence)
        }
    }

    @Test
    fun unanchoredReferenceHasNoAnchor() {
        val r = VerdictJson.parse(Verdicts.WITH_CHANGES.replace(Verdicts.ANCHOR, "null"))
        assertNull(r.reference?.anchor)
    }

    @Test
    fun malformedVerdictsAreRejected() {
        val bad = listOf(
            "{}",
            Verdicts.NO_RECORD.replace("NO_RECORD", "MAYBE"),
            Verdicts.NO_RECORD.replace("\"flags\":0", "\"flags\":128"),
            Verdicts.NO_RECORD.replace("\"reasonBits\":4608", "\"reasonBits\":32768"),
            Verdicts.WITH_CHANGES.replace("\"exact_hash\"", "\"vibes\""),
            Verdicts.WITH_CHANGES.replace("\"recordId\":\"${Verdicts.RECORD_ID}\",\"claim\"", "\"recordId\":\"x\",\"claim\""),
        )
        for (text in bad) assertFailsWith<WireException>(text) { VerdictJson.parse(text) }
    }
}
