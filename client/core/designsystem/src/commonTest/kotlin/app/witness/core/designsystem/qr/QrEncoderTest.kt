package app.witness.core.designsystem.qr

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class QrEncoderTest {
    @Test
    fun modeSelectionPicksMostCompactMode() {
        assertEquals(QrMode.Numeric, QrEncoder.modeFor("0123456789"))
        assertEquals(QrMode.Alphanumeric, QrEncoder.modeFor("HELLO WORLD"))
        assertEquals(QrMode.Alphanumeric, QrEncoder.modeFor("HTTPS://WITNESS.APP/P/H4IL-7QK"))
        assertEquals(QrMode.Byte, QrEncoder.modeFor("witness.app/p/H4IL-7QK"))
        assertEquals(QrMode.Byte, QrEncoder.modeFor("Ünïcode ✓"))
        assertEquals(QrMode.Byte, QrEncoder.modeFor(""))
    }

    @Test
    fun payloadBitsFollowModeRules() {
        assertEquals(0, QrEncoder.payloadBits(QrMode.Numeric, 0))
        assertEquals(4, QrEncoder.payloadBits(QrMode.Numeric, 1))
        assertEquals(7, QrEncoder.payloadBits(QrMode.Numeric, 2))
        assertEquals(10, QrEncoder.payloadBits(QrMode.Numeric, 3))
        assertEquals(27, QrEncoder.payloadBits(QrMode.Numeric, 8))
        assertEquals(6, QrEncoder.payloadBits(QrMode.Alphanumeric, 1))
        assertEquals(61, QrEncoder.payloadBits(QrMode.Alphanumeric, 11))
        assertEquals(80, QrEncoder.payloadBits(QrMode.Byte, 10))
    }

    @Test
    fun characterCountBitsSwitchAtVersionTenAndTwentySeven() {
        assertEquals(listOf(10, 10, 12, 12, 14, 14), listOf(1, 9, 10, 26, 27, 40).map { QrMode.Numeric.characterCountBits(it) })
        assertEquals(listOf(9, 11, 13), listOf(9, 26, 40).map { QrMode.Alphanumeric.characterCountBits(it) })
        assertEquals(listOf(8, 16, 16), listOf(9, 10, 40).map { QrMode.Byte.characterCountBits(it) })
    }

    @Test
    fun versionOneCapacitiesMatchStandard() {
        assertEquals(1, QrEncoder.minimumVersion(QrMode.Numeric, 41, QrEcc.L))
        assertEquals(2, QrEncoder.minimumVersion(QrMode.Numeric, 42, QrEcc.L))
        assertEquals(1, QrEncoder.minimumVersion(QrMode.Alphanumeric, 25, QrEcc.L))
        assertEquals(2, QrEncoder.minimumVersion(QrMode.Alphanumeric, 26, QrEcc.L))
        assertEquals(1, QrEncoder.minimumVersion(QrMode.Byte, 17, QrEcc.L))
        assertEquals(2, QrEncoder.minimumVersion(QrMode.Byte, 18, QrEcc.L))
        assertEquals(1, QrEncoder.minimumVersion(QrMode.Byte, 14, QrEcc.M))
        assertEquals(1, QrEncoder.minimumVersion(QrMode.Byte, 7, QrEcc.H))
        assertEquals(2, QrEncoder.minimumVersion(QrMode.Byte, 8, QrEcc.H))
    }

    @Test
    fun versionFortyCapacitiesMatchStandard() {
        assertEquals(40, QrEncoder.minimumVersion(QrMode.Byte, 2953, QrEcc.L))
        assertNull(QrEncoder.minimumVersion(QrMode.Byte, 2954, QrEcc.L))
        assertEquals(40, QrEncoder.minimumVersion(QrMode.Numeric, 7089, QrEcc.L))
        assertNull(QrEncoder.minimumVersion(QrMode.Numeric, 7090, QrEcc.L))
        assertEquals(40, QrEncoder.minimumVersion(QrMode.Alphanumeric, 1852, QrEcc.H))
        assertNull(QrEncoder.minimumVersion(QrMode.Alphanumeric, 1853, QrEcc.H))
    }

    @Test
    fun tooMuchDataFails() {
        assertFailsWith<IllegalArgumentException> { QrEncoder.encodeBytes(ByteArray(2954), QrEcc.L) }
    }

    @Test
    fun invalidArgumentsFail() {
        assertFailsWith<IllegalArgumentException> { QrEncoder.encode("A", mask = 8) }
        assertFailsWith<IllegalArgumentException> { QrEncoder.encode("A", mask = -2) }
        assertFailsWith<IllegalArgumentException> { QrEncoder.encode("A", minVersion = 0) }
        assertFailsWith<IllegalArgumentException> { QrEncoder.encode("A", minVersion = 41) }
    }

    @Test
    fun proofLinkFitsSmallSymbol() {
        val qr = QrEncoder.encode("https://witness.app/p/H4IL-7QK")
        assertEquals(QrMode.Byte, qr.mode)
        assertEquals(3, qr.version)
        assertEquals(29, qr.size)
        assertEquals(QrEcc.M, qr.ecc)
    }

    @Test
    fun minVersionIsHonoured() {
        assertEquals(5, QrEncoder.encode("A", minVersion = 5).version)
    }

    @Test
    fun boostRaisesErrorCorrectionWhenItFits() {
        val qr = QrEncoder.encode("HELLO", QrEcc.L, boostEcc = true)
        assertEquals(1, qr.version)
        assertEquals(QrEcc.H, qr.ecc)
        assertEquals(QrEcc.L, QrEncoder.encode("HELLO", QrEcc.L).ecc)
    }

    @Test
    fun interleaveProducesStandardCodewordsForHelloWorldOneM() {
        val data = ByteArray(16) { intArrayOf(32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17)[it].toByte() }
        val out = QrEncoder.interleave(data, 1, QrEcc.M)
        assertEquals(26, out.size)
        assertContentEquals(data, out.copyOfRange(0, 16))
        assertContentEquals(intArrayOf(196, 35, 39, 119, 235, 215, 231, 226, 93, 23), out.copyOfRange(16, 26).map { it.toInt() and 0xFF }.toIntArray())
    }

    @Test
    fun interleaveMixesBlocksForMultiBlockVersions() {
        val data = ByteArray(QrTables.dataCodewords(5, QrEcc.Q)) { it.toByte() }
        val out = QrEncoder.interleave(data, 5, QrEcc.Q)
        assertEquals(QrTables.rawDataModules(5) / 8, out.size)
        assertEquals(listOf(0, 15, 30, 46, 1), out.take(5).map { it.toInt() and 0xFF })
    }

    @Test
    fun functionPatternsArePresent() {
        val qr = QrEncoder.encode("WITNESS", QrEcc.Q)
        val n = qr.size
        for ((cx, cy) in listOf(3 to 3, n - 4 to 3, 3 to n - 4)) {
            assertTrue(qr[cx, cy])
            assertTrue(!qr[cx - 2, cy] && !qr[cx + 2, cy])
            assertTrue(qr[cx - 3, cy - 3] && qr[cx + 3, cy + 3])
        }
        for (i in 8 until n - 8) {
            assertEquals(i % 2 == 0, qr[6, i])
            assertEquals(i % 2 == 0, qr[i, 6])
        }
        assertTrue(qr[8, n - 8])
    }

    @Test
    fun formatInformationEncodesLevelAndMask() {
        for (ecc in QrEcc.entries) for (mask in 0..7) {
            val qr = QrEncoder.encode("FORMAT", ecc, mask = mask)
            assertEquals(mask, qr.mask)
            var bits = 0
            for (i in 0..5) if (qr[8, i]) bits = bits or (1 shl i)
            if (qr[8, 7]) bits = bits or (1 shl 6)
            if (qr[8, 8]) bits = bits or (1 shl 7)
            if (qr[7, 8]) bits = bits or (1 shl 8)
            for (i in 9 until 15) if (qr[14 - i, 8]) bits = bits or (1 shl i)
            assertEquals(QrTables.formatBits(ecc, mask), bits, "$ecc/$mask")
            var copy = 0
            for (i in 0 until 8) if (qr[qr.size - 1 - i, 8]) copy = copy or (1 shl i)
            for (i in 8 until 15) if (qr[8, qr.size - 15 + i]) copy = copy or (1 shl i)
            assertEquals(bits, copy)
        }
    }

    @Test
    fun versionInformationIsDrawnFromVersionSeven() {
        val qr = QrEncoder.encode("A", minVersion = 7)
        var bits = 0
        for (i in 0 until 18) if (qr[qr.size - 11 + i % 3, i / 3]) bits = bits or (1 shl i)
        assertEquals(QrTables.versionBits(7), bits)
        var mirrored = 0
        for (i in 0 until 18) if (qr[i / 3, qr.size - 11 + i % 3]) mirrored = mirrored or (1 shl i)
        assertEquals(bits, mirrored)
    }

    @Test
    fun encodingIsDeterministic() {
        val a = QrEncoder.encode("https://case.witness.app/v/8F2K-QM7D")
        val b = QrEncoder.encode("https://case.witness.app/v/8F2K-QM7D")
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
        assertEquals(a.toString(), b.toString())
    }

    @Test
    fun differentMasksProduceDifferentSymbols() {
        val symbols = (0..7).map { QrEncoder.encode("MASKS", mask = it).toString() }
        assertEquals(8, symbols.toSet().size)
    }

    @Test
    fun autoMaskIsOneOfEight() {
        repeat(20) { assertTrue(QrEncoder.encode("ITEM $it").mask in 0..7) }
    }

    @Test
    fun darknessIsBalanced() {
        val qr = QrEncoder.encode("https://witness.app/p/H4IL-7QK")
        val ratio = qr.darkModules().toDouble() / (qr.size * qr.size)
        assertTrue(ratio in 0.35..0.65, "ratio=$ratio")
    }

    @Test
    fun emptyAndUnicodeInputsEncode() {
        assertEquals(1, QrEncoder.encode("").version)
        val unicode = QrEncoder.encode("Witness ✓ — Zürich")
        assertEquals(QrMode.Byte, unicode.mode)
    }

    @Test
    fun largestSymbolEncodes() {
        val qr = QrEncoder.encodeBytes(ByteArray(2953) { (it * 31).toByte() }, QrEcc.L)
        assertEquals(40, qr.version)
        assertEquals(177, qr.size)
    }

    @Test
    fun toStringRendersRows() {
        val rows = QrEncoder.encode("1").toString().split('\n')
        assertEquals(21, rows.size)
        assertTrue(rows.all { it.length == 21 })
        assertTrue(rows[0].startsWith("#######."))
    }
}
