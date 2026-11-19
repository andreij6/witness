package app.witness.core.icp

import app.witness.core.domain.FixedClock
import app.witness.core.icp.cases.CaseChunks
import app.witness.core.icp.cases.CaseOpenResult
import app.witness.core.icp.cases.CaseOpener
import app.witness.core.icp.cases.CaseSealer
import app.witness.core.icp.fake.InMemoryCaseStore
import app.witness.core.icp.id.CaseId
import app.witness.core.icp.web.WebCaseCrypto
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class WebCaseCryptoTest {
    private val crypto = WebCaseCrypto()

    @Test
    fun gcmVectorMatchesAndroid() = runTest {
        val sealed = crypto.sealChunk(GcmVector.KEY, GcmVector.NONCE, GcmVector.PLAIN, 0, GcmVector.PLAIN.size, GcmVector.AAD)
        assertEquals(GcmVector.CHUNK_HEX, sealed.toHex())
        assertContentEquals(GcmVector.PLAIN, crypto.openChunk(GcmVector.KEY, sealed, GcmVector.AAD))
    }

    @Test
    fun openRejectsTampering() = runTest {
        val chunk = hex(GcmVector.CHUNK_HEX)
        chunk[20] = (chunk[20].toInt() xor 1).toByte()
        assertNull(crypto.openChunk(GcmVector.KEY, chunk, GcmVector.AAD))
        assertNull(crypto.openChunk(GcmVector.KEY, ByteArray(CaseChunks.OVERHEAD - 1), GcmVector.AAD))
    }

    @Test
    fun sha256Vectors() = runTest {
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", crypto.sha256(ByteArray(0)).toHex())
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", crypto.sha256("abc".encodeToByteArray()).toHex())
    }

    @Test
    fun randomBytesHaveRequestedLength() {
        assertEquals(32, crypto.randomBytes(32).size)
    }

    @Test
    fun caseRoundTripWithWebCrypto() = runTest {
        val store = InMemoryCaseStore()
        val bundle = ByteArray(CaseChunks.PLAIN_CHUNK + 77) { (it * 7).toByte() }
        val sealed = CaseSealer(crypto).seal(CaseId.of(4, ByteArray(15) { 1 }), bundle, 2_000)
        store.upload(sealed)
        val opened = CaseOpener(store, crypto, FixedClock(1_000)).open(sealed.link)
        assertIs<CaseOpenResult.Opened>(opened)
        assertContentEquals(bundle, opened.bundle)
    }

    @Test
    fun webServicesHaveNoUploader() {
        assertNull(createIcpServices(IcpEndpoint.localReplica("aaaaa-aa")).uploader)
    }
}
