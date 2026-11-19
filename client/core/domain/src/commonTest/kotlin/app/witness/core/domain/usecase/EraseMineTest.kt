package app.witness.core.domain.usecase

import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EraseMineTest {
    @Test
    fun messageLayout() {
        val id = RecordId.fromBytes(ByteArray(10) { 1 })
        val msg = EraseMine.message(id, ByteArray(16) { 2 })
        assertContentEquals("witness/erase/v1".encodeToByteArray() + ByteArray(10) { 1 } + ByteArray(16) { 2 }, msg)
    }

    @Test
    fun erasesOriginalAndPublishedDerivatives() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val root = g.sealOne()
        val post = g.post.publish(g.post.draft(root.recordId))
        g.anchor()
        val r = g.erase.publicCopiesOf(root.recordId)
        assertEquals(setOf(root.recordId, post.recordId), r.erased.toSet())
        assertTrue(r.complete)
        assertTrue(g.captures.get(root.recordId)!!.publicErased)
        assertTrue(g.captures.get(post.recordId)!!.publicErased)
        g.registry.erased.forEach { req ->
            assertTrue(FakeKeyVault.verify(req.keyId, EraseMine.message(req.recordId, req.nonce.toByteArray()), req.signature))
            assertEquals(EraseMine.NONCE_SIZE, req.nonce.size)
        }
        assertEquals(emptyList(), g.erase.publicCopiesOf(root.recordId).erased)
    }

    @Test
    fun skipsUnanchoredAndImports() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val queued = g.sealOne()
        val imp = RecordId.fromBytes(ByteArray(10) { 9 })
        g.captures.insert(Capture(imp, null, CaptureOrigin.IMPORT, AnchoringStatus.ANCHORED, 0, null, LocationConfidence.LOW, Dims(1, 1)), null, null)
        val r = g.erase.everything()
        assertTrue(r.erased.isEmpty())
        assertTrue(g.registry.erased.isEmpty())
        assertFalse(EraseMine.isErasable(queued))
        assertEquals(EraseReport(), g.erase.publicCopiesOf(RecordId.fromBytes(ByteArray(10))))
    }

    @Test
    fun missingKeyAndRejectionAreReported() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val a = g.sealAnchored()
        g.vault.forget(a.keyRef!!.keyId)
        val r = g.erase.everything()
        assertEquals(listOf(a.recordId), r.missingKey)
        assertFalse(r.complete)
    }

    @Test
    fun rejectedByRegistry() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val a = g.sealAnchored()
        val bogus = a.copy(recordId = RecordId.fromBytes(ByteArray(10) { 7 }))
        g.captures.insert(bogus, null, null)
        val r = g.erase.everything()
        assertEquals(listOf(bogus.recordId), r.rejected)
        assertEquals(listOf(a.recordId), r.erased)
    }

    @Test
    fun offlineStopsAndReportsPartialProgress() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val a = g.sealAnchored()
        g.registry.offline = true
        val r = g.erase.everything()
        assertTrue(r.offline)
        assertFalse(r.complete)
        assertTrue(r.erased.isEmpty())
        assertFalse(g.captures.get(a.recordId)!!.publicErased)
    }
}
