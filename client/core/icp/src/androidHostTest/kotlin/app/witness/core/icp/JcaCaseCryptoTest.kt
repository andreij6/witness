package app.witness.core.icp

import app.witness.core.domain.FixedClock
import app.witness.core.icp.cases.CaseChunks
import app.witness.core.icp.cases.CaseOpenResult
import app.witness.core.icp.cases.CaseOpener
import app.witness.core.icp.cases.CaseSealer
import app.witness.core.icp.id.CaseId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class JcaCaseCryptoTest {
    private val crypto = JcaCaseCrypto()

    @Test
    fun gcmVectorTestCase16() = runTest {
        val sealed = crypto.sealChunk(GcmVector.KEY, GcmVector.NONCE, GcmVector.PLAIN, 0, GcmVector.PLAIN.size, GcmVector.AAD)
        assertEquals(GcmVector.CHUNK_HEX, sealed.toHex())
        assertContentEquals(GcmVector.PLAIN, crypto.openChunk(GcmVector.KEY, sealed, GcmVector.AAD))
    }

    @Test
    fun sealsSubrange() = runTest {
        val padded = byteArrayOf(9, 9) + GcmVector.PLAIN + byteArrayOf(9)
        val sealed = crypto.sealChunk(GcmVector.KEY, GcmVector.NONCE, padded, 2, 2 + GcmVector.PLAIN.size, GcmVector.AAD)
        assertEquals(GcmVector.CHUNK_HEX, sealed.toHex())
    }

    @Test
    fun openRejectsTamperingAndShortInput() = runTest {
        val chunk = hex(GcmVector.CHUNK_HEX)
        for (i in chunk.indices step 7) {
            val bad = chunk.copyOf().also { it[i] = (it[i].toInt() xor 1).toByte() }
            assertNull(crypto.openChunk(GcmVector.KEY, bad, GcmVector.AAD), "byte $i")
        }
        assertNull(crypto.openChunk(GcmVector.KEY, chunk, GcmVector.AAD + 0))
        assertNull(crypto.openChunk(GcmVector.KEY, chunk.copyOf(CaseChunks.OVERHEAD - 1), GcmVector.AAD))
        assertNull(crypto.openChunk(ByteArray(32), chunk, GcmVector.AAD))
    }

    @Test
    fun emptyPlaintextRoundTrips() = runTest {
        val sealed = crypto.sealChunk(GcmVector.KEY, GcmVector.NONCE, ByteArray(0), 0, 0, ByteArray(0))
        assertEquals(CaseChunks.OVERHEAD, sealed.size)
        assertContentEquals(ByteArray(0), crypto.openChunk(GcmVector.KEY, sealed, ByteArray(0)))
    }

    @Test
    fun sha256Vectors() = runTest {
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", crypto.sha256(ByteArray(0)).toHex())
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", crypto.sha256("abc".encodeToByteArray()).toHex())
    }

    @Test
    fun randomBytesHaveRequestedLength() {
        assertEquals(0, crypto.randomBytes(0).size)
        assertEquals(32, crypto.randomBytes(32).size)
    }

    @Test
    fun caseRoundTripWithRealCrypto() = runTest {
        val services = createIcpServices(IcpEndpoint.localReplica("aaaaa-aa"))
        val uploader = assertNotNull(services.uploader)
        val bundle = ByteArray(CaseChunks.PLAIN_CHUNK + 1234) { (it * 31).toByte() }
        val caseId = CaseId.of(0, ByteArray(15) { 3 })
        val sealed = CaseSealer(services.crypto).seal(caseId, bundle, 2_000)
        uploader.upload(sealed)
        val opened = CaseOpener(services.cases, services.crypto, FixedClock(1_000)).open(sealed.link)
        assertIs<CaseOpenResult.Opened>(opened)
        assertContentEquals(bundle, opened.bundle)
        val parsed = assertNotNull(app.witness.core.icp.cases.CaseLink.parse(sealed.link.url()))
        assertIs<CaseOpenResult.Opened>(services.opener(FixedClock(1_000)).open(parsed))
    }
}
