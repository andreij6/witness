package app.witness.nativelib

import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.PdqPosting
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.UploadRequest
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.Entitlement
import app.witness.core.model.RecordId
import kotlinx.coroutines.test.runTest
import uniffi.witness_android.CaseInfo
import uniffi.witness_android.CertifiedRecord
import uniffi.witness_android.MediaErr
import uniffi.witness_android.PdqHit
import uniffi.witness_android.ProbeResult
import uniffi.witness_android.UploadTarget
import uniffi.witness_android.WitnessException
import java.security.MessageDigest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NativeNetworkTest {
    private val h = Harness()
    private val fetched = ArrayList<String>()
    private var page: ByteArray? = null
    private val media = NativeMedia(h.witness, h.keys, "http://10.0.2.2:8090", { url -> fetched += url; page.takeIf { url.contains("media-1") } })
    private val case = CaseId.of(1, ByteArray(15) { 4 })
    private val request = UploadRequest(case, 2, 100, ByteString.EMPTY, null)

    @Test
    fun probeSendsPdqAndExactAndMapsHits() = runTest {
        h.witness.probeAnswer = ProbeResult(listOf(PdqHit(4uL, 9u)), listOf(1uL), listOf(2uL, 3uL), true, 10uL)
        val r = NativeIndex(h.witness).probe(ShardInfo(0, 0), ProbeQuery(listOf(ByteString.of(byteArrayOf(1))), listOf(ByteString.of(byteArrayOf(2)))))
        assertEquals(listOf(PdqPosting(4, 9)), r.pdq)
        assertEquals(listOf(2L, 3L), r.exact)
        assertTrue(r.truncated)
        val (pdq, wm, exact) = h.witness.probes.single()
        assertContentEquals(byteArrayOf(1), pdq.single())
        assertTrue(wm.isEmpty())
        assertContentEquals(byteArrayOf(2), exact.single())
        assertEquals(emptyList(), NativeIndex(h.witness).derivatives(ShardInfo(0, 0), 1))
    }

    @Test
    fun eachCaseChunkIsASignedUploadOnTheCaseShard() = runTest {
        h.enrolledKey()
        val t = media.begin(request)
        assertEquals(1, t.shard)
        media.putChunk(t, 0, byteArrayOf(1, 2))
        media.putChunk(t, 1, byteArrayOf(3))
        media.commit(t)
        val (target, digest) = h.witness.begun[0]
        assertEquals(UploadTarget.CaseChunk(case.text, 0u, 2u), target)
        assertContentEquals(MessageDigest.getInstance("SHA-256").digest(byteArrayOf(1, 2)), digest)
        assertEquals(UploadTarget.CaseChunk(case.text, 1u, 2u), h.witness.begun[1].first)
        assertEquals(2, h.witness.chunks.size)
        assertEquals(h.witness.chunks.map { it.first }, h.witness.commits)
        assertFailsWith<IllegalArgumentException> { media.commit(t) }
    }

    @Test
    fun previewIsOneSignedUploadOnTheRegistrysMediaShard() = runTest {
        val key = h.enrolledKey()
        val record = RecordId.fromBytes(ByteArray(10) { 3 })
        h.witness.records[record.text] = certified(record, mediaShard = 0u)
        val shrunk = NativeMedia(h.witness, h.keys, "http://gw", { null }) { it.copyOf(2) }
        shrunk.publish(record, key, ByteString.of(byteArrayOf(5, 6, 7, 8)))
        val (target, digest) = h.witness.begun.single()
        assertEquals(UploadTarget.Preview(record.text), target)
        assertEquals(listOf<UByte>(0u), h.witness.beganShards)
        assertContentEquals(MessageDigest.getInstance("SHA-256").digest(byteArrayOf(5, 6)), digest)
        assertContentEquals(byteArrayOf(5, 6), h.witness.chunks.single().second)
        assertEquals(1, h.witness.commits.size)
    }

    @Test
    fun previewFollowsEachRecordsMediaShardNotTheLastShard() = runTest {
        val key = h.enrolledKey()
        val a = RecordId.fromBytes(ByteArray(10) { 1 })
        val b = RecordId.fromBytes(ByteArray(10) { 2 })
        h.witness.records[a.text] = certified(a, mediaShard = 1u)
        h.witness.records[b.text] = certified(b, mediaShard = 0u)
        media.publish(a, key, ByteString.of(byteArrayOf(1)))
        media.publish(b, key, ByteString.of(byteArrayOf(2)))
        assertEquals(listOf<UByte>(1u, 0u), h.witness.beganShards)
        assertEquals(listOf<UByte>(0u, 1u), h.witness.shards.media.map { it.shard }.sorted())
    }

    @Test
    fun previewOfAnUnanchoredRecordWaitsInsteadOfGuessingAShard() = runTest {
        val key = h.enrolledKey()
        val record = RecordId.fromBytes(ByteArray(10) { 7 })
        assertIs<OfflineException>(assertFailsWith<Exception> { media.publish(record, key, ByteString.of(byteArrayOf(1))) })
        assertTrue(h.witness.begun.isEmpty())
    }

    @Test
    fun caseExpiryComesFromMediaCaseInfoOnTheCaseShard() = runTest {
        val asked = ArrayList<Pair<UByte, String>>()
        val withInfo = NativeMedia(h.witness, h.keys, "http://gw", { null }, caseInfo = { s, id -> asked += s to id; CaseInfo(5_000_000_000uL, 10uL, 1u) })
        assertEquals(5_000L, withInfo.caseExpiry(case))
        assertEquals(listOf<Pair<UByte, String>>(1.toUByte() to case.text), asked)
        assertNull(media.caseExpiry(case))
        assertNull(NativeMedia(h.witness, h.keys, "http://gw", { null }, caseInfo = { _, _ -> null }).caseExpiry(case))
        val offline = NativeMedia(h.witness, h.keys, "http://gw", { null }, caseInfo = { _, _ -> throw WitnessException.Network("down") })
        assertNull(offline.caseExpiry(case))
        val refused = NativeMedia(h.witness, h.keys, "http://gw", { null }, caseInfo = { _, _ -> throw WitnessException.Media(MediaErr.Malformed("case id")) })
        assertNull(refused.caseExpiry(case))
    }

    @Test
    fun caseUploadsAcceptUpToThePlusChunkCeiling() = runTest {
        h.enrolledKey()
        val t = media.begin(request.copy(chunkCount = NativeMedia.MAX_CASE_CHUNKS))
        media.putChunk(t, 255, byteArrayOf(1))
        assertEquals(UploadTarget.CaseChunk(case.text, 255u, 256u), h.witness.begun.single().first)
        assertEquals(256, NativeMedia.MAX_CASE_CHUNKS)
    }

    @Test
    fun casesWithoutAGrantAreCappedAtTheFreeCanisterLimit() {
        assertEquals(Entitlement.FREE_CASE_BYTES, media.maxCaseBytes)
        assertEquals(16L * MediaGateway.MAX_CHUNK_BYTES, media.maxCaseBytes)
    }

    @Test
    fun previewIsPublishedOnlyWhenMediaServesIt() = runTest {
        val record = RecordId.fromBytes(ByteArray(10) { 3 })
        assertEquals(false, media.isPublished(record))
        page = byteArrayOf(1)
        assertEquals(true, media.isPublished(record))
        assertTrue(fetched.all { it.startsWith("http://10.0.2.2:8090/p/${record.text}?canisterId=") })
    }

    @Test
    fun previewRejectsOversizedOrEmptyOutputAndMapsMediaErrors() = runTest {
        val key = h.enrolledKey()
        val record = RecordId.fromBytes(ByteArray(10) { 3 })
        val big = NativeMedia(h.witness, h.keys, "http://gw", { null }) { ByteArray(NativeMedia.MAX_PREVIEW_BYTES + 1) }
        assertFailsWith<IllegalArgumentException> { big.publish(record, key, ByteString.of(byteArrayOf(1))) }
        assertFailsWith<IllegalArgumentException> { media.publish(record, key, ByteString.EMPTY) }
        h.witness.failure = WitnessException.Media(MediaErr.Unauthorized)
        assertEquals("media.unauthorized", assertFailsWith<NativeException> { media.publish(record, key, ByteString.of(byteArrayOf(1))) }.code)
        assertTrue(h.witness.begun.isEmpty())
    }

    @Test
    fun alreadyUploadedChunksSkipThePut() = runTest {
        h.enrolledKey()
        h.witness.missing = emptyList()
        val t = media.begin(request)
        media.putChunk(t, 0, byteArrayOf(1))
        assertTrue(h.witness.chunks.isEmpty())
        assertEquals(1, h.witness.commits.size)
    }

    @Test
    fun uploadsNeedAnEnrolledKeyAndValidChunks() = runTest {
        val t = media.begin(request)
        assertEquals("no_key", assertFailsWith<NativeException> { media.putChunk(t, 0, byteArrayOf(1)) }.code)
        h.enrolledKey()
        assertFailsWith<IllegalArgumentException> { media.putChunk(t, 2, byteArrayOf(1)) }
        assertFailsWith<IllegalArgumentException> { media.putChunk(t, 0, ByteArray(0)) }
        assertFailsWith<IllegalArgumentException> { media.putChunk(t, 0, ByteArray(MediaGateway.MAX_CHUNK_BYTES + 1)) }
        assertFailsWith<IllegalArgumentException> { media.begin(request.copy(chunkCount = 0)) }
        assertFailsWith<IllegalArgumentException> { media.begin(request.copy(chunkCount = 257)) }
        assertFailsWith<IllegalArgumentException> { media.putChunk(t.copy(uploadId = "nope"), 0, byteArrayOf(1)) }
    }

    @Test
    fun mediaErrorsMapToDomainErrors() = runTest {
        h.enrolledKey()
        val t = media.begin(request)
        h.witness.failure = WitnessException.Media(MediaErr.QuotaExceeded)
        assertEquals("media.quota_exceeded", assertFailsWith<NativeException> { media.putChunk(t, 0, byteArrayOf(1)) }.code)
        h.witness.failure = WitnessException.Media(MediaErr.RetryAfterNs(1uL))
        assertIs<OfflineException>(assertFailsWith<Exception> { media.putChunk(t, 0, byteArrayOf(1)) })
    }

    @Test
    fun referenceTriesEveryMediaShardThroughTheGateway() = runTest {
        val id = RecordId.parse("WERC8GMRZGE196QV")
        assertNull(media.reference(id))
        assertEquals(
            listOf("http://10.0.2.2:8090/p/WERC8GMRZGE196QV?canisterId=media-0", "http://10.0.2.2:8090/p/WERC8GMRZGE196QV?canisterId=media-1"),
            fetched,
        )
        page = byteArrayOf(-1, -40)
        assertEquals(ByteString.of(byteArrayOf(-1, -40)), media.reference(id))
    }

    private fun certified(record: RecordId, mediaShard: UByte) =
        CertifiedRecord(1uL, record.text, byteArrayOf(), byteArrayOf(), byteArrayOf(), 1uL, emptyList(), 2uL, byteArrayOf(), mediaShard)
}
