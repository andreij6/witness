package app.witness.core.icp.fake

import app.witness.core.icp.cases.CaseLink
import app.witness.core.icp.cases.CaseMeta
import app.witness.core.icp.cases.SealedCase
import app.witness.core.icp.id.CaseId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class InMemoryCaseStoreTest {
    private val caseId = CaseId.of(0, ByteArray(15))
    private val meta = CaseMeta(2, 10, ByteArray(32) { 1 }, 99)
    private val sealed = SealedCase(CaseLink(caseId, ByteArray(32)), meta, listOf(byteArrayOf(1, 2), byteArrayOf(3)))

    @Test
    fun uploadThenRead() = runTest {
        val store = InMemoryCaseStore()
        assertNull(store.meta(caseId))
        store.upload(sealed)
        assertEquals(meta, store.meta(caseId))
        assertContentEquals(byteArrayOf(1, 2), store.chunk(caseId, 0))
        assertContentEquals(byteArrayOf(3), store.chunk(caseId, 1))
        assertNull(store.chunk(caseId, 2))
        assertNull(store.chunk(caseId, -1))
    }

    @Test
    fun returnsDefensiveCopies() = runTest {
        val store = InMemoryCaseStore()
        store.upload(sealed)
        sealed.chunks[0][0] = 42
        store.chunk(caseId, 0)!![1] = 42
        assertContentEquals(byteArrayOf(1, 2), store.chunk(caseId, 0))
        sealed.chunks[0][0] = 1
    }

    @Test
    fun eraseRemovesCase() = runTest {
        val store = InMemoryCaseStore()
        store.upload(sealed)
        assertTrue(store.erase(caseId))
        assertFalse(store.erase(caseId))
        assertNull(store.meta(caseId))
    }

    @Test
    fun metaEqualityAndHashCopy() {
        val hash = ByteArray(32) { 5 }
        val m = CaseMeta(1, 1, hash, 1)
        hash[0] = 0
        assertTrue(m.hashEquals(ByteArray(32) { 5 }))
        m.bundleHash[0] = 0
        assertTrue(m.hashEquals(ByteArray(32) { 5 }))
        assertEquals(m, CaseMeta(1, 1, ByteArray(32) { 5 }, 1))
        assertEquals(m.hashCode(), CaseMeta(1, 1, ByteArray(32) { 5 }, 1).hashCode())
    }
}
