package app.witness.core.icp.cases

import app.witness.core.icp.id.CaseId
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CaseLinkTest {
    private val caseId = CaseId.of(2, ByteArray(15) { (it + 1).toByte() })
    private val key = ByteArray(32) { it.toByte() }
    private val link = CaseLink(caseId, key)

    @Test
    fun urlFormat() {
        assertEquals("https://case.witness.app/v/${caseId.text}#k=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8", link.url())
        assertTrue(link.url("http://localhost:8080").startsWith("http://localhost:8080/v/"))
    }

    @Test
    fun urlRoundTrips() {
        assertEquals(link, CaseLink.parse(link.url()))
        assertEquals(link, CaseLink.parse(link.url("http://127.0.0.1:4943")))
    }

    @Test
    fun parseToleratesExtraFragmentParamsQueryAndTrailingSlash() {
        val b64 = link.url().substringAfter("#k=")
        assertEquals(link, CaseLink.parse("https://case.witness.app/v/${caseId.text}/?utm=x#a=1&k=$b64&b=2"))
        assertEquals(link, CaseLink.parse("https://case.witness.app/v/${caseId.text.lowercase()}#k=$b64"))
        assertEquals(link, CaseLink.parse("/v/${caseId.text}", "#k=$b64"))
        assertEquals(link, CaseLink.parse("/v/${caseId.text}", "k=$b64="))
    }

    @Test
    fun parseRejectsMalformed() {
        val b64 = link.url().substringAfter("#k=")
        for (url in listOf(
            "https://case.witness.app/v/${caseId.text}",
            "https://case.witness.app/v/${caseId.text}#",
            "https://case.witness.app/v/${caseId.text}#x=$b64",
            "https://case.witness.app/v/${caseId.text}#k=${b64.dropLast(4)}",
            "https://case.witness.app/v/${caseId.text}#k=$b64" + "AAAA",
            "https://case.witness.app/v/${caseId.text}#k=!!!!",
            "https://case.witness.app/p/${caseId.text}#k=$b64",
            "https://case.witness.app/v/NOTACASE#k=$b64",
            "",
        )) {
            assertNull(CaseLink.parse(url), url)
        }
    }

    @Test
    fun keyIsDefensivelyCopiedAndRedacted() {
        val raw = key.copyOf()
        val l = CaseLink(caseId, raw)
        raw[0] = 99
        assertContentEquals(key, l.key)
        l.key[1] = 99
        assertContentEquals(key, l.key)
        assertFalse("AAEC" in l.toString())
        assertTrue("redacted" in l.toString())
    }

    @Test
    fun equalityUsesKeyContents() {
        assertEquals(link, CaseLink(caseId, key.copyOf()))
        assertEquals(link.hashCode(), CaseLink(caseId, key.copyOf()).hashCode())
        assertNotEquals(link, CaseLink(caseId, ByteArray(32)))
    }

    @Test
    fun rejectsWrongKeySize() {
        assertFailsWith<IllegalArgumentException> { CaseLink(caseId, ByteArray(31)) }
        assertNull(CaseLink.keyFrom("k=AAAA"))
    }
}
