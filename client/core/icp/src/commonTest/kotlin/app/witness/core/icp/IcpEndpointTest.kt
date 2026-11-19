package app.witness.core.icp

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class IcpEndpointTest {
    @Test
    fun localReplicaFetchesRootKey() {
        val e = IcpEndpoint.localReplica("uxrrr-q7777-77774-qaaaq-cai")
        assertTrue(e.fetchRootKey)
        assertTrue(e.host.startsWith("http://127.0.0.1"))
        assertEquals("uxrrr-q7777-77774-qaaaq-cai", e.registryId)
    }

    @Test
    fun mainnetNeverFetchesRootKey() {
        val e = IcpEndpoint.mainnet("x")
        assertFalse(e.fetchRootKey)
        assertTrue(e.host.startsWith("https://"))
    }
}
