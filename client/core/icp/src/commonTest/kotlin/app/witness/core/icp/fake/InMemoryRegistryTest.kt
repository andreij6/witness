package app.witness.core.icp.fake

import app.witness.core.icp.id.CaptureId
import app.witness.core.icp.id.RecordId
import app.witness.core.icp.registry.CertifiedRecord
import app.witness.core.icp.registry.ShardInfo
import app.witness.core.icp.registry.ShardMap
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class InMemoryRegistryTest {
    private val id = RecordId.of(ByteArray(10) { 1 })
    private val record = CertifiedRecord(1, id, byteArrayOf(), byteArrayOf(), byteArrayOf(), 2, emptyList(), 3, byteArrayOf())

    @Test
    fun servesConfiguredData() = runTest {
        val capture = CaptureId.of(5, 1)!!
        val map = ShardMap.of(listOf(ShardInfo("a", 0, 0)), emptyList())
        val registry = InMemoryRegistry(listOf(record), mapOf(capture to listOf(id)), map)
        assertEquals(record, registry.get(id))
        assertNull(registry.get(RecordId.of(ByteArray(10))))
        assertEquals(listOf(id), registry.byCapture(capture))
        assertEquals(emptyList(), registry.byCapture(CaptureId.of(6, 1)!!))
        assertEquals(map, registry.shards())
    }
}
