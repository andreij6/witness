package app.witness.core.domain.usecase

import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.port.CaseBundle
import app.witness.core.domain.port.CaseBundleItem
import app.witness.core.model.ByteString
import app.witness.core.model.RecordId
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class CaseBundleCborTest {
    private val item = CaseBundleItem(RecordId.fromBytes(ByteArray(10) { 1 }), CaseVector.JPEG_A, null)

    @Test
    fun encodingMatchesTheRustWitnessWebVector() {
        assertEquals(CaseVector.BUNDLE, ByteString.of(CaseBundleCbor.encode(CaseVector.BUNDLE_VALUE)).hex())
    }

    @Test
    fun bundleHashMatchesTheRustVector() {
        assertEquals(CaseVector.HASH, ByteString.of(SoftwareSha256.digest(CaseBundleCbor.encode(CaseVector.BUNDLE_VALUE))).hex())
    }

    @Test
    fun recordIdOfTheVectorSubmissionIsItsClaimDigest() {
        assertEquals(RecordId.fromDigest(SoftwareSha256.digest("claim".encodeToByteArray())), CaseVector.SUBMISSION.recordId)
    }

    @Test
    fun aadMatchesTheRustLayout() {
        val aad = ShareCase.aad(CaseVector.CASE_ID.toBytes(), 0, 1, ByteString.ofHex(CaseVector.HASH).toByteArray())
        assertEquals(15 + 16 + 4 + 4 + 32, aad.size)
        assertContentEquals("witness/case/v1".encodeToByteArray(), aad.copyOf(15))
        assertEquals(CaseVector.CASE_ID_TEXT, CaseVector.CASE_ID.text)
    }

    @Test
    fun fragmentKeyMatchesTheRustEncoding() {
        val link = TestGraph().share.link(CaseVector.CASE_ID, CaseVector.KEY)
        assertEquals("https://case.witness.app/v/${CaseVector.CASE_ID_TEXT}${CaseVector.FRAGMENT}", link)
    }

    @Test
    fun summaryIsOptional() {
        val bare = CaseBundleCbor.encode(CaseBundle("", null, listOf(item)))
        assertEquals(0xa3, bare[0].toInt() and 0xff)
        assertEquals(0x60, bare[4].toInt() and 0xff)
    }

    @Test
    fun encodingIsDeterministic() {
        assertContentEquals(CaseBundleCbor.encode(CaseVector.BUNDLE_VALUE), CaseBundleCbor.encode(CaseVector.BUNDLE_VALUE))
    }

    @Test
    fun largeImagesUseFourByteLengths() {
        val big = ByteString.of(ByteArray(70_000) { 5 })
        val out = CaseBundleCbor.encode(CaseBundle("t", null, listOf(item.copy(jpeg = big))))
        val at = out.size - big.size - 5
        assertEquals(0x5a, out[at].toInt() and 0xff)
        assertEquals(70_000, ((out[at + 1].toInt() and 0xff) shl 24) or ((out[at + 2].toInt() and 0xff) shl 16) or ((out[at + 3].toInt() and 0xff) shl 8) or (out[at + 4].toInt() and 0xff))
    }

    @Test
    fun rejectsBundlesTheRustDecoderWouldRefuse() {
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("t", null, emptyList())) }
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("t", null, List(65) { item })) }
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("x".repeat(513), null, listOf(item))) }
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("t", "s".repeat(16 * 1024 + 1), listOf(item))) }
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("t", null, listOf(item.copy(jpeg = ByteString.EMPTY)))) }
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("t", null, listOf(item.copy(submission = CaseVector.SUBMISSION)))) }
    }

    @Test
    fun acceptsTheLimits() {
        CaseBundleCbor.encode(CaseBundle("x".repeat(512), "s".repeat(16 * 1024), List(64) { item }))
    }

    @Test
    fun multiByteTitlesCountBytesNotChars() {
        assertFailsWith<IllegalArgumentException> { CaseBundleCbor.encode(CaseBundle("é".repeat(257), null, listOf(item))) }
    }
}
