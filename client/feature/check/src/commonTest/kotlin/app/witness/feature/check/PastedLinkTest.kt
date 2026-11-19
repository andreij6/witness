package app.witness.feature.check

import app.witness.core.domain.port.Links
import app.witness.core.model.CaseId
import kotlin.io.encoding.Base64
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class PastedLinkTest {
    private val id = Fixtures.recordId
    private val caseId = CaseId.of(3, ByteArray(15) { (it * 7).toByte() })
    private val key = Base64.UrlSafe.withPadding(Base64.PaddingOption.ABSENT).encode(ByteArray(32) { it.toByte() })

    private fun proof(text: String) = assertIs<PastedLink.Proof>(PastedLink.parse(text)).recordId

    @Test
    fun proofLinksInEveryCommonShape() {
        assertEquals(id, proof("https://witness.app/p/${id.text}"))
        assertEquals(id, proof("http://witness.app/p/${id.text}"))
        assertEquals(id, proof("witness.app/p/${id.text}"))
        assertEquals(id, proof("https://www.witness.app/p/${id.text}/"))
        assertEquals(id, proof("  https://witness.app/p/${id.text}?utm=x#top \n"))
        assertEquals(id, proof("HTTPS://WITNESS.APP/p/${id.text}"))
        assertEquals(id, proof("https://witness.app/p/${id.grouped()}"))
    }

    @Test
    fun bareRecordIdIsAccepted() {
        assertEquals(id, proof(id.text))
        assertEquals(id, proof(id.grouped()))
    }

    @Test
    fun caseLinkCarriesIdAndFragment() {
        val link = assertIs<PastedLink.Case>(PastedLink.parse("https://case.witness.app/v/${caseId.text}#k=$key"))
        assertEquals(caseId, link.caseId)
        assertEquals("k=$key", link.fragment)
    }

    @Test
    fun caseLinkToStringRedactsKey() {
        val link = PastedLink.parse("https://case.witness.app/v/${caseId.text}#k=$key")
        assertTrue(key !in link.toString())
    }

    @Test
    fun caseLinksWithoutAValidKeyAreRejected() {
        assertEquals(PastedLink.Problem.MissingKey, PastedLink.parse("https://case.witness.app/v/${caseId.text}"))
        assertEquals(PastedLink.Problem.MissingKey, PastedLink.parse("https://case.witness.app/v/${caseId.text}#k=short"))
        assertEquals(PastedLink.Problem.MissingKey, PastedLink.parse("https://case.witness.app/v/${caseId.text}#k=${key}AA"))
        assertEquals(PastedLink.Problem.MissingKey, PastedLink.parse("https://case.witness.app/v/${caseId.text}#k=!!!"))
    }

    @Test
    fun malformedWitnessIdsAreReported() {
        assertEquals(PastedLink.Problem.Malformed, PastedLink.parse("https://witness.app/p/"))
        assertEquals(PastedLink.Problem.Malformed, PastedLink.parse("https://witness.app/p/NOT-AN-ID"))
        assertEquals(PastedLink.Problem.Malformed, PastedLink.parse("https://case.witness.app/v/xyz#k=$key"))
    }

    @Test
    fun foreignAndEmptyInputs() {
        assertEquals(PastedLink.Problem.Empty, PastedLink.parse("   "))
        assertEquals(PastedLink.Problem.Foreign, PastedLink.parse("https://instagram.com/p/abc"))
        assertEquals(PastedLink.Problem.Foreign, PastedLink.parse("www.example.com"))
        assertEquals(PastedLink.Problem.Foreign, PastedLink.parse("hello there"))
        assertEquals(PastedLink.Problem.Foreign, PastedLink.parse("https://evil.app/witness.app/p/${id.text}"))
    }

    @Test
    fun oversizedInputIsRejectedWithoutParsing() {
        assertEquals(PastedLink.Problem.Malformed, PastedLink.parse("https://witness.app/p/" + "A".repeat(PastedLink.MAX_LENGTH)))
    }

    @Test
    fun customBasesAreHonoured() {
        val links = Links(proofBase = "https://staging.witness.test/p/", caseBase = "https://case.staging.witness.test/v/")
        assertEquals(id, assertIs<PastedLink.Proof>(PastedLink.parse("https://staging.witness.test/p/${id.text}", links)).recordId)
        assertEquals(PastedLink.Problem.Foreign, PastedLink.parse("https://witness.app/p/${id.text}", links))
    }

    @Test
    fun everyProblemHasAMessage() {
        PastedLink.Problem.entries.forEach { assertTrue(it.message.isNotBlank()) }
        assertNotEquals(PastedLink.Problem.Empty.message, PastedLink.Problem.Foreign.message)
    }

    @Test
    fun caseEqualityUsesIdAndFragment() {
        assertEquals(PastedLink.Case(caseId, "k=a"), PastedLink.Case(caseId, "k=a"))
        assertNotEquals(PastedLink.Case(caseId, "k=a"), PastedLink.Case(caseId, "k=b"))
    }
}
