package app.witness.feature.settings

import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.RecordId
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class DeleteAccountTest {
    private val ids = listOf(RecordId.fromBytes(ByteArray(10) { 1 }), RecordId.fromBytes(ByteArray(10) { 2 }))

    @Test
    fun completeErasureUnlinksIdentity() = runTest {
        val identity = InMemoryIdentityAccount(TestPrincipals.alice)
        val outcome = DeleteAccount({ EraseReport(erased = ids) }, identity)()
        assertEquals(DeleteOutcome.Deleted(2), outcome)
        assertNull(identity.value)
        assertEquals(1, identity.unlinkCount)
    }

    @Test
    fun deletesEvenWithNothingToErase() = runTest {
        val identity = InMemoryIdentityAccount()
        assertEquals(DeleteOutcome.Deleted(0), DeleteAccount({ EraseReport() }, identity)())
        assertEquals(1, identity.unlinkCount)
    }

    @Test
    fun incompleteErasureKeepsLinkForRetryFromWeb() = runTest {
        val identity = InMemoryIdentityAccount(TestPrincipals.alice)
        val report = EraseReport(erased = ids.take(1), rejected = ids.drop(1))
        assertEquals(DeleteOutcome.Incomplete(report), DeleteAccount({ report }, identity)())
        assertEquals(TestPrincipals.alice, identity.value)
        assertEquals(0, identity.unlinkCount)
    }

    @Test
    fun offlineErasureKeepsLink() = runTest {
        val identity = InMemoryIdentityAccount(TestPrincipals.alice)
        val outcome = DeleteAccount({ EraseReport(offline = true) }, identity)()
        assertEquals(DeleteOutcome.Incomplete(EraseReport(offline = true)), outcome)
        assertEquals(0, identity.unlinkCount)
    }

    @Test
    fun offlineUnlinkIsIncomplete() = runTest {
        val identity = InMemoryIdentityAccount(TestPrincipals.alice, offline = true)
        assertEquals(DeleteOutcome.Incomplete(EraseReport(erased = ids, offline = true)), DeleteAccount({ EraseReport(erased = ids) }, identity)())
    }

    @Test
    fun unexpectedErasureFailurePropagates() = runTest {
        assertFailsWith<IllegalStateException> { DeleteAccount({ error("boom") }, InMemoryIdentityAccount())() }
    }

    @Test
    fun erasesExactlyOnce() = runTest {
        var calls = 0
        DeleteAccount({ calls++; EraseReport() }, InMemoryIdentityAccount())()
        assertEquals(1, calls)
    }

    @Test
    fun offlineExceptionFromEraseIsNotSwallowed() = runTest {
        assertFailsWith<OfflineException> { DeleteAccount({ throw OfflineException() }, InMemoryIdentityAccount())() }
    }
}
