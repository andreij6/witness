package app.witness.core.icp.registry

import app.witness.core.icp.id.RecordId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class CertifiedRecordTest {
    private fun record(proof: List<ByteArray> = listOf(byteArrayOf(1), byteArrayOf(2)), claim: ByteArray = byteArrayOf(9)) = CertifiedRecord(
        recordIx = 7,
        recordId = RecordId.of(ByteArray(10) { it.toByte() }),
        claim = claim,
        c2paClaim = byteArrayOf(3),
        cose = byteArrayOf(4),
        anchoredNs = 1_700_000_000_000_000_000,
        mmrProof = proof,
        leafCount = 8,
        certificate = byteArrayOf(5),
    )

    @Test
    fun equalityComparesContents() {
        assertEquals(record(), record())
        assertEquals(record().hashCode(), record().hashCode())
        assertNotEquals(record(), record(claim = byteArrayOf(8)))
        assertNotEquals(record(), record(proof = listOf(byteArrayOf(1))))
        assertNotEquals(record(), record(proof = listOf(byteArrayOf(1), byteArrayOf(3))))
    }

    @Test
    fun toStringOmitsPayloads() {
        val s = record().toString()
        assertTrue("ix=7" in s)
        assertTrue("claim" !in s)
    }
}
