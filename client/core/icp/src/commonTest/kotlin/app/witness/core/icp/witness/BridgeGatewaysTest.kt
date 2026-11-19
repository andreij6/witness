package app.witness.core.icp.witness

import app.witness.core.icp.witness.WitnessFixtures.KEY
import app.witness.core.icp.witness.WitnessFixtures.KEY_HEX
import app.witness.core.icp.witness.WitnessFixtures.PROOF
import app.witness.core.icp.witness.WitnessFixtures.RECORD_ID
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.KeyId
import app.witness.core.model.RecordId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class BridgeGatewaysTest {
    private val id = RecordId.parse(RECORD_ID)
    private val keyId = KeyId.fromBytes(ByteString.ofHex(KEY_HEX).toByteArray())

    @Test
    fun registryReadsVerifiedRecordsAndKeys() = runTest {
        val bridge = FakeBridge(proofs = mapOf(RECORD_ID to PROOF), keys = mapOf(KEY_HEX to KEY))
        val registry = BridgeRegistry(bridge)
        assertEquals(7L, registry.get(id)?.recordIx)
        assertEquals(keyId, registry.getKey(keyId)?.keyId)
        assertEquals(listOf("proof:$RECORD_ID", "key:$KEY_HEX"), bridge.calls)
    }

    @Test
    fun registryReturnsNullForUnknownIds() = runTest {
        val registry = BridgeRegistry(FakeBridge())
        assertNull(registry.get(id))
        assertNull(registry.getKey(keyId))
    }

    @Test
    fun registryPropagatesBridgeAndWireFailures() = runTest {
        assertFailsWith<IllegalStateException> { BridgeRegistry(FakeBridge(failure = IllegalStateException("offline"))).get(id) }
        assertFailsWith<WireException> { BridgeRegistry(FakeBridge(proofs = mapOf(RECORD_ID to "{}"))).get(id) }
    }

    @Test
    fun registryWritesAreRefused() = runTest {
        val registry = BridgeRegistry(FakeBridge())
        assertFailsWith<UnsupportedOperationException> { registry.challenge() }
        assertFailsWith<UnsupportedOperationException> { registry.submit(emptyList()) }
        assertFailsWith<UnsupportedOperationException> { registry.byCapture(CaptureId.fromLong(1)!!) }
        assertFailsWith<UnsupportedOperationException> { registry.shards() }
        assertFailsWith<UnsupportedOperationException> { registry.getByIndex(1) }
    }

    @Test
    fun mediaReturnsPreviewBytes() = runTest {
        val media = BridgeMedia(FakeBridge(previews = mapOf(RECORD_ID to byteArrayOf(1, 2))))
        assertEquals(ByteString.of(byteArrayOf(1, 2)), media.reference(id))
        assertNull(BridgeMedia(FakeBridge()).reference(id))
        assertFailsWith<UnsupportedOperationException> { media.commit(app.witness.core.domain.port.UploadTicket(0, "x")) }
        assertFailsWith<UnsupportedOperationException> { BridgeIndex.derivatives(app.witness.core.domain.port.ShardInfo(0, 0), 1) }
    }
}
