package app.witness.core.designsystem.qr

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class QrTablesTest {
    @Test
    fun sizesGrowByFourModules() {
        assertEquals(21, QrTables.size(1))
        assertEquals(25, QrTables.size(2))
        assertEquals(177, QrTables.size(40))
    }

    @Test
    fun rawModuleCountsMatchStandard() {
        assertEquals(208, QrTables.rawDataModules(1))
        assertEquals(359, QrTables.rawDataModules(2))
        assertEquals(1568, QrTables.rawDataModules(7))
        assertEquals(29648, QrTables.rawDataModules(40))
    }

    @Test
    fun dataCodewordCapacitiesMatchStandard() {
        assertEquals(listOf(19, 16, 13, 9), QrEcc.entries.map { QrTables.dataCodewords(1, it) })
        assertEquals(listOf(34, 28, 22, 16), QrEcc.entries.map { QrTables.dataCodewords(2, it) })
        assertEquals(listOf(156, 124, 88, 66), QrEcc.entries.map { QrTables.dataCodewords(7, it) })
        assertEquals(listOf(2956, 2334, 1666, 1276), QrEcc.entries.map { QrTables.dataCodewords(40, it) })
    }

    @Test
    fun codewordTotalsAreConsistentForEveryVersion() {
        for (v in 1..40) for (ecc in QrEcc.entries) {
            val total = QrTables.rawDataModules(v) / 8
            val ecTotal = QrTables.eccPerBlock(v, ecc) * QrTables.blocks(v, ecc)
            assertEquals(total, QrTables.dataCodewords(v, ecc) + ecTotal, "v$v $ecc")
            assertTrue(QrTables.dataCodewords(v, ecc) > 0)
        }
    }

    @Test
    fun capacityIsMonotonicInVersionAndInverseInLevel() {
        for (ecc in QrEcc.entries) for (v in 2..40) assertTrue(QrTables.dataCodewords(v, ecc) > QrTables.dataCodewords(v - 1, ecc))
        for (v in 1..40) {
            val caps = listOf(QrEcc.L, QrEcc.M, QrEcc.Q, QrEcc.H).map { QrTables.dataCodewords(v, it) }
            assertEquals(caps.sortedDescending(), caps)
        }
    }

    @Test
    fun alignmentPositionsMatchStandard() {
        assertContentEquals(intArrayOf(), QrTables.alignmentPositions(1))
        assertContentEquals(intArrayOf(6, 18), QrTables.alignmentPositions(2))
        assertContentEquals(intArrayOf(6, 22, 38), QrTables.alignmentPositions(7))
        assertContentEquals(intArrayOf(6, 26, 48, 70), QrTables.alignmentPositions(15))
        assertContentEquals(intArrayOf(6, 34, 60, 86, 112, 138), QrTables.alignmentPositions(32))
        assertContentEquals(intArrayOf(6, 30, 58, 86, 114, 142, 170), QrTables.alignmentPositions(40))
    }

    @Test
    fun formatBitsMatchStandardTable() {
        assertEquals(0x77C4, QrTables.formatBits(QrEcc.L, 0))
        assertEquals(0x5412, QrTables.formatBits(QrEcc.M, 0))
        assertEquals(0x355F, QrTables.formatBits(QrEcc.Q, 0))
        assertEquals(0x1689, QrTables.formatBits(QrEcc.H, 0))
        assertEquals(0x45F9, QrTables.formatBits(QrEcc.M, 4))
        assertEquals(0x40CE, QrTables.formatBits(QrEcc.M, 5))
        assertEquals(0x4AA0, QrTables.formatBits(QrEcc.M, 7))
        assertEquals(0x083B, QrTables.formatBits(QrEcc.H, 7))
    }

    @Test
    fun formatBitsAreDistinctForAllThirtyTwoCombinations() {
        val all = QrEcc.entries.flatMap { ecc -> (0..7).map { QrTables.formatBits(ecc, it) } }
        assertEquals(32, all.toSet().size)
        all.forEach { assertTrue(it in 0 until (1 shl 15)) }
    }

    @Test
    fun versionBitsMatchStandardTable() {
        assertEquals(0x07C94, QrTables.versionBits(7))
        assertEquals(0x085BC, QrTables.versionBits(8))
        assertEquals(0x1F250, QrTables.versionBits(31))
        assertEquals(0x28C69, QrTables.versionBits(40))
    }
}
