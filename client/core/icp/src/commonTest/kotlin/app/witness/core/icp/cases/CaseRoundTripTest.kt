package app.witness.core.icp.cases

import app.witness.core.domain.FixedClock
import app.witness.core.icp.XorShift
import app.witness.core.icp.fake.InMemoryCaseStore
import app.witness.core.icp.id.CaseId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class CaseRoundTripTest {
    private val now = 1_700_000_000_000L
    private val caseId = CaseId.of(1, ByteArray(15) { 7 })
    private val crypto = ToyCaseCrypto()

    private class TamperingStore(private val inner: CaseStore, private val chunkFn: (Int, ByteArray?) -> ByteArray?) : CaseStore {
        var metaOverride: CaseMeta? = null
        override suspend fun meta(caseId: CaseId): CaseMeta? = metaOverride ?: inner.meta(caseId)
        override suspend fun chunk(caseId: CaseId, ix: Int): ByteArray? = chunkFn(ix, inner.chunk(caseId, ix))
    }

    private suspend fun sealed(size: Int, store: InMemoryCaseStore = InMemoryCaseStore()): Pair<SealedCase, InMemoryCaseStore> {
        val bundle = XorShift(size.toLong() + 1).bytes(size)
        val s = CaseSealer(crypto).seal(caseId, bundle, now + 60_000)
        store.upload(s)
        return s to store
    }

    private fun opener(store: CaseStore, at: Long = now) = CaseOpener(store, crypto, FixedClock(at))

    @Test
    fun roundTripsAcrossChunkBoundaries() = runTest {
        for (size in listOf(0, 1, CaseChunks.PLAIN_CHUNK - 1, CaseChunks.PLAIN_CHUNK, CaseChunks.PLAIN_CHUNK + 1, 2 * CaseChunks.PLAIN_CHUNK + 5)) {
            val (s, store) = sealed(size)
            assertEquals(CaseChunks.count(size.toLong()), s.chunks.size)
            assertEquals(CaseChunks.wireSize(size.toLong()), s.chunks.sumOf { it.size.toLong() })
            val result = opener(store).open(s.link)
            assertIs<CaseOpenResult.Opened>(result, "size $size")
            assertContentEquals(XorShift(size.toLong() + 1).bytes(size), result.bundle)
        }
    }

    @Test
    fun sealUsesFreshKeyAndNoncePerChunk() = runTest {
        val bundle = ByteArray(CaseChunks.PLAIN_CHUNK * 2 + 1)
        val a = CaseSealer(crypto).seal(caseId, bundle, now)
        val b = CaseSealer(crypto).seal(caseId, bundle, now)
        assertNotEquals(a.link, b.link)
        val nonces = a.chunks.map { it.copyOfRange(0, CaseChunks.NONCE_BYTES).toList() }.toSet()
        assertEquals(a.chunks.size, nonces.size)
    }

    @Test
    fun metaDescribesBundle() = runTest {
        val (s, _) = sealed(CaseChunks.PLAIN_CHUNK + 3)
        assertEquals(2, s.meta.count)
        assertEquals(CaseChunks.PLAIN_CHUNK + 3L, s.meta.bytes)
        assertEquals(now + 60_000, s.meta.expiresMs)
        assertEquals(32, s.meta.bundleHash.size)
    }

    @Test
    fun rejectsOversizedBundle() = runTest {
        assertFailsWith<IllegalArgumentException> { CaseSealer(crypto).seal(caseId, ByteArray(16 * CaseChunks.PLAIN_CHUNK + 1), now) }
    }

    @Test
    fun missingCaseIsNotFound() = runTest {
        val (s, _) = sealed(10)
        assertEquals(CaseOpenResult.NotFound, opener(InMemoryCaseStore()).open(s.link))
    }

    @Test
    fun expiredCaseIsReportedBeforeFetchingChunks() = runTest {
        val (s, store) = sealed(10)
        val guard = TamperingStore(store) { _, _ -> error("must not fetch") }
        assertEquals(CaseOpenResult.Expired, opener(guard, at = now + 60_000).open(s.link))
    }

    @Test
    fun flippedByteIsTampered() = runTest {
        val (s, store) = sealed(CaseChunks.PLAIN_CHUNK + 100)
        val bad = TamperingStore(store) { ix, c -> if (ix == 1) c!!.also { it[40] = (it[40] + 1).toByte() } else c }
        assertEquals(CaseOpenResult.Tampered, opener(bad).open(s.link))
    }

    @Test
    fun missingChunkIsTampered() = runTest {
        val (s, store) = sealed(CaseChunks.PLAIN_CHUNK + 100)
        assertEquals(CaseOpenResult.Tampered, opener(TamperingStore(store) { ix, c -> if (ix == 0) null else c }).open(s.link))
    }

    @Test
    fun swappedChunksFailAadBinding() = runTest {
        val (s, store) = sealed(2 * CaseChunks.PLAIN_CHUNK)
        val swapped = TamperingStore(store) { ix, _ -> s.chunks[1 - ix] }
        assertEquals(CaseOpenResult.Tampered, opener(swapped).open(s.link))
    }

    @Test
    fun truncatedChunkIsTampered() = runTest {
        val (s, store) = sealed(100)
        assertEquals(CaseOpenResult.Tampered, opener(TamperingStore(store) { _, c -> c!!.copyOf(c.size - 1) }).open(s.link))
    }

    @Test
    fun wrongKeyIsTampered() = runTest {
        val (s, store) = sealed(100)
        assertEquals(CaseOpenResult.Tampered, opener(store).open(CaseLink(caseId, ByteArray(32))))
    }

    @Test
    fun inconsistentMetaIsTampered() = runTest {
        val (s, store) = sealed(100)
        for (meta in listOf(
            CaseMeta(2, 100, s.meta.bundleHash, now + 1),
            CaseMeta(1, 99, s.meta.bundleHash, now + 1),
            CaseMeta(1, 100, ByteArray(32), now + 1),
            CaseMeta(17, 17L * CaseChunks.PLAIN_CHUNK, s.meta.bundleHash, now + 1),
            CaseMeta(1, -1, s.meta.bundleHash, now + 1),
        )) {
            val store2 = TamperingStore(store) { _, c -> c }.also { it.metaOverride = meta }
            assertEquals(CaseOpenResult.Tampered, opener(store2).open(s.link), meta.toString())
        }
    }

    @Test
    fun chunksFromAnotherCaseAreRejected() = runTest {
        val (s, store) = sealed(100)
        val other = CaseSealer(crypto).seal(CaseId.of(1, ByteArray(15) { 8 }), XorShift(101).bytes(100), now + 60_000)
        val mixed = TamperingStore(store) { _, _ -> other.chunks[0] }
        assertEquals(CaseOpenResult.Tampered, opener(mixed).open(s.link))
    }

    @Test
    fun openedBundleIsDeterministic() = runTest {
        val (s, store) = sealed(5_000)
        val a = opener(store).open(s.link) as CaseOpenResult.Opened
        val b = opener(store).open(s.link) as CaseOpenResult.Opened
        assertTrue(a.bundle.contentEquals(b.bundle))
    }
}
