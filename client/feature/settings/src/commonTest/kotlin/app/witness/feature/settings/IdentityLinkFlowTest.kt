package app.witness.feature.settings

import app.witness.core.domain.fake.SeededRandom
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IdentityLinkFlowTest {
    private val account = InMemoryIdentityAccount()
    private val flow = IdentityLinkFlow(account, SeededRandom(11))
    private val alice = TestPrincipals.alice

    private fun nonceIn(url: String): String = url.substringAfter("nonce=").substringBefore('&')

    private fun authorized(nonce: String, principal: Principal = alice) = "${IiLinkContract.REDIRECT}?v=1&nonce=$nonce&principal=${principal.text}"

    @Test
    fun startsIdle() = assertEquals(LinkStatus.Idle, flow.status.value)

    @Test
    fun beginWaitsWithFreshNonce() {
        val first = flow.begin()
        assertEquals(LinkStatus.Waiting, flow.status.value)
        assertTrue(first.startsWith(IiLinkContract.AUTHORIZE_URL))
        assertEquals(IiLinkContract.NONCE_BYTES * 2, nonceIn(first).length)
        assertNotEquals(nonceIn(first), nonceIn(flow.begin()))
    }

    @Test
    fun completesAndLinksAccount() = runTest {
        val nonce = nonceIn(flow.begin())
        assertEquals(LinkStatus.Linked(alice), flow.complete(authorized(nonce)))
        assertEquals(LinkStatus.Linked(alice), flow.status.value)
        assertEquals(alice, account.principal.first())
    }

    @Test
    fun callbackCannotBeReplayed() = runTest {
        val uri = authorized(nonceIn(flow.begin()))
        flow.complete(uri)
        assertEquals(LinkStatus.Failed(LinkError.NO_PENDING_REQUEST), flow.complete(uri))
    }

    @Test
    fun staleNonceFromEarlierBeginIsRejected() = runTest {
        val stale = nonceIn(flow.begin())
        val fresh = nonceIn(flow.begin())
        assertEquals(LinkStatus.Failed(LinkError.NONCE_MISMATCH), flow.complete(authorized(stale)))
        assertEquals(LinkStatus.Linked(alice), flow.complete(authorized(fresh)))
    }

    @Test
    fun forgedCallbackDoesNotCancelPendingRequest() = runTest {
        val nonce = nonceIn(flow.begin())
        flow.complete(authorized("0".repeat(32), TestPrincipals.bob))
        assertNull(account.value)
        assertEquals(LinkStatus.Linked(alice), flow.complete(authorized(nonce)))
    }

    @Test
    fun cancellationReturnsToIdle() = runTest {
        val nonce = nonceIn(flow.begin())
        assertEquals(LinkStatus.Idle, flow.complete("${IiLinkContract.REDIRECT}?v=1&nonce=$nonce&error=cancelled"))
        assertNull(account.value)
    }

    @Test
    fun registryRejectionIsReported() = runTest {
        account.accept = false
        assertEquals(LinkStatus.Failed(LinkError.REJECTED), flow.complete(authorized(nonceIn(flow.begin()))))
        assertNull(account.value)
    }

    @Test
    fun offlineIsReported() = runTest {
        account.offline = true
        assertEquals(LinkStatus.Failed(LinkError.OFFLINE), flow.complete(authorized(nonceIn(flow.begin()))))
    }

    @Test
    fun malformedCallbackClearsPending() = runTest {
        val nonce = nonceIn(flow.begin())
        assertEquals(LinkStatus.Failed(LinkError.MALFORMED), flow.complete("${IiLinkContract.REDIRECT}?v=1&nonce=$nonce"))
        assertEquals(LinkStatus.Failed(LinkError.NO_PENDING_REQUEST), flow.complete(authorized(nonce)))
    }

    @Test
    fun resetForgetsPendingRequest() = runTest {
        val nonce = nonceIn(flow.begin())
        flow.reset()
        assertEquals(LinkStatus.Idle, flow.status.value)
        assertEquals(LinkStatus.Failed(LinkError.NO_PENDING_REQUEST), flow.complete(authorized(nonce)))
    }

    @Test
    fun customAuthorizeUrlIsUsed() {
        val local = IdentityLinkFlow(account, SeededRandom(1), IiLinkContract("http://localhost:4943/link"))
        assertTrue(local.begin().startsWith("http://localhost:4943/link?v=1&nonce="))
    }

    @Test
    fun sameSeedGivesSameRequest() {
        val a = IdentityLinkFlow(InMemoryIdentityAccount(), SeededRandom(5)).begin()
        val b = IdentityLinkFlow(InMemoryIdentityAccount(), SeededRandom(5)).begin()
        assertEquals(a, b)
    }
}
