package app.witness.nav

import app.witness.core.model.CaseId
import app.witness.core.model.RecordId
import app.witness.core.platform.share.ShareParser
import app.witness.core.platform.share.SharedItem
import app.witness.feature.check.CheckKey
import app.witness.feature.check.CheckSharedKey
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ProofPageKey
import kotlin.io.encoding.Base64
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DeepLinksTest {
    private val record = RecordId.fromDigest(ByteArray(32) { (it * 3).toByte() })
    private val caseId = CaseId.of(2, ByteArray(CaseId.RANDOM_SIZE) { (it * 5).toByte() })
    private val key = Base64.UrlSafe.withPadding(Base64.PaddingOption.ABSENT).encode(ByteArray(32) { it.toByte() })

    @Test
    fun proofLinkOpensTheProofPage() {
        assertEquals(ProofPageKey(record.text), DeepLinks.keyFor("https://witness.app/p/${record.text}"))
        assertEquals(ProofPageKey(record.text), DeepLinks.keyFor("https://witness.app/p/${record.text}?utm=x#y"))
    }

    @Test
    fun caseLinkCarriesItsKeyFragment() {
        val k = assertIs<CaseLinkKey>(DeepLinks.keyFor("https://case.witness.app/v/${caseId.text}#k=$key"))
        assertEquals(caseId.text, k.caseId)
        assertEquals("k=$key", k.fragment)
    }

    @Test
    fun caseLinkWithoutKeyIsRejected() {
        assertNull(DeepLinks.keyFor("https://case.witness.app/v/${caseId.text}"))
        assertNull(DeepLinks.keyFor("https://case.witness.app/v/${caseId.text}#k=short"))
    }

    @Test
    fun checkLinksInCommonShapes() {
        listOf(
            "https://witness.app/check",
            "https://witness.app/check/",
            "https://www.witness.app/check?from=x",
            "HTTPS://WITNESS.APP/CHECK#top",
        ).forEach { assertEquals(CheckKey, DeepLinks.keyFor(it), it) }
    }

    @Test
    fun lookalikeCheckLinksAreNotChecks() {
        listOf(
            "https://witness.app/checks",
            "https://evilwitness.app/check",
            "https://witness.app.evil.com/check",
            "witness.app/check",
            "https://witness.app/check/x",
            "",
        ).forEach { assertFalse(DeepLinks.isCheck(it), it) }
    }

    @Test
    fun foreignAndMalformedLinksOpenNothing() {
        listOf(
            "https://example.com/p/${record.text}",
            "https://witness.app/p/not-an-id",
            "https://witness.app/",
            "hello",
            "",
            "https://witness.app/p/" + "A".repeat(5000),
        ).forEach { assertNull(DeepLinks.keyFor(it), it) }
    }

    @Test
    fun sharedProofLinkOpensTheProofPage() {
        val item = SharedItem.ProofLink("https://witness.app/p/${record.text}", record)
        assertEquals(ProofPageKey(record.text), DeepLinks.keyFor(item, { error("not an image") }))
    }

    @Test
    fun sharedProofLinksFromForeignHostsOpenNothing() {
        listOf(
            "https://example.com/p/${record.text}",
            "https://witness.app.evil.com/p/${record.text}",
            "https://evil.com/witness.app/p/${record.text}",
        ).forEach { url ->
            val item = assertIs<SharedItem.ProofLink>(ShareParser.parse("text/plain", url, emptyList()), url)
            assertNull(DeepLinks.keyFor(item, { error("not an image") }), url)
        }
    }

    @Test
    fun sharedGenericLinkIsParsed() {
        assertEquals(CheckKey, DeepLinks.keyFor(SharedItem.Link("https://witness.app/check"), { null }))
        assertNull(DeepLinks.keyFor(SharedItem.Link("https://example.com"), { null }))
    }

    @Test
    fun sharedImagesStashOnlyTheFirstAndOpenCheck() {
        val stashed = ArrayList<String>()
        val key = DeepLinks.keyFor(SharedItem.Images(listOf("content://a", "content://b")), { stashed += it; "7" })
        assertEquals(CheckSharedKey("7"), key)
        assertEquals(listOf("content://a"), stashed)
    }

    @Test
    fun sharedImagesWithoutStreamsOrStashOpenNothing() {
        assertNull(DeepLinks.keyFor(SharedItem.Images(emptyList()), { "x" }))
        assertNull(DeepLinks.keyFor(SharedItem.Images(listOf("content://a")), { null }))
    }

    @Test
    fun keyForIsDeterministic() {
        val url = "https://witness.app/p/${record.text}"
        assertTrue((1..5).map { DeepLinks.keyFor(url) }.distinct().size == 1)
    }
}
