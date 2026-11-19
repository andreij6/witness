package app.witness.core.platform.crypto

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class EcdsaSignatureTest {
    private fun der(r: ByteArray, s: ByteArray): ByteArray {
        val body = byteArrayOf(0x02, r.size.toByte()) + r + byteArrayOf(0x02, s.size.toByte()) + s
        return byteArrayOf(0x30, body.size.toByte()) + body
    }

    private val one = byteArrayOf(1)
    private val two = byteArrayOf(2)

    @Test
    fun smallIntegersArePaddedLeft() {
        val raw = EcdsaSignature.derToRaw(der(one, two))
        assertEquals(64, raw.size)
        assertEquals(1, raw[31].toInt())
        assertEquals(2, raw[63].toInt())
        assertTrue(raw.copyOfRange(0, 31).all { it == 0.toByte() })
    }

    @Test
    fun leadingZeroForHighBitIsStripped() {
        val r = byteArrayOf(0) + ByteArray(32) { 0x7f }.also { it[0] = 0x80.toByte() }
        val raw = EcdsaSignature.derToRaw(der(r, one))
        assertContentEquals(r.copyOfRange(1, 33), raw.copyOfRange(0, 32))
    }

    @Test
    fun highSIsNormalizedToLowS() {
        val n = hex("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551")
        val nMinusOne = n.copyOf().also { it[31] = 0x50 }
        val raw = EcdsaSignature.derToRaw(der(one, byteArrayOf(0) + nMinusOne))
        assertEquals(1, raw[63].toInt())
        assertTrue(raw.copyOfRange(32, 63).all { it == 0.toByte() })
        assertTrue(EcdsaSignature.isLowS(raw))
    }

    @Test
    fun halfOrderIsKept() {
        val half = hex("7fffffff800000007fffffffffffffffde737d56d38bcf4279dce5617e3192a8")
        val raw = EcdsaSignature.derToRaw(der(one, half))
        assertContentEquals(half, raw.copyOfRange(32, 64))
    }

    @Test
    fun rawToDerRoundTrips() {
        val samples = listOf(der(one, two), der(byteArrayOf(0x7f), byteArrayOf(0, 0x80.toByte())), der(ByteArray(32) { 0x11 }, ByteArray(32) { 0x22 }))
        for (d in samples) assertContentEquals(d, EcdsaSignature.rawToDer(EcdsaSignature.derToRaw(d)))
    }

    @Test
    fun rawToDerRejectsWrongSize() {
        assertFailsWith<IllegalArgumentException> { EcdsaSignature.rawToDer(ByteArray(63)) }
    }

    @Test
    fun malformedDerIsRejected() {
        val bad = listOf(
            ByteArray(0),
            byteArrayOf(0x31, 6, 2, 1, 1, 2, 1, 1),
            der(one, two).copyOf(9) + byteArrayOf(0),
            der(one, two).also { it[1] = 7 },
            der(byteArrayOf(0x80.toByte()), one),
            der(byteArrayOf(0, 1), one),
            der(ByteArray(0), one),
            der(byteArrayOf(0), one),
            der(one, ByteArray(34) { 1 }),
            der(byteArrayOf(1) + ByteArray(32), one),
            der(hex("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551").let { byteArrayOf(0) + it }, one),
            der(one, two).also { it[2] = 0x03 },
        )
        for (b in bad) assertFailsWith<IllegalArgumentException>(b.joinToString()) { EcdsaSignature.derToRaw(b) }
    }

    @Test
    fun deterministic() {
        val d = der(ByteArray(32) { 3 }, ByteArray(32) { 0x7f })
        assertContentEquals(EcdsaSignature.derToRaw(d), EcdsaSignature.derToRaw(d))
    }

    private fun hex(s: String) = ByteArray(s.length / 2) { s.substring(2 * it, 2 * it + 2).toInt(16).toByte() }
}

class PreimagesTest {
    @Test
    fun offlineChallengeLayout() {
        val p = Preimages.offlineChallenge(byteArrayOf(9, 8), 0x0102030405060708)
        assertEquals("witness-offline".length + 2 + 8, p.size)
        assertEquals("witness-offline", p.copyOfRange(0, 15).decodeToString())
        assertContentEquals(byteArrayOf(9, 8), p.copyOfRange(15, 17))
        assertContentEquals(byteArrayOf(1, 2, 3, 4, 5, 6, 7, 8), p.copyOfRange(17, 25))
    }

    @Test
    fun offlineChallengeNeedsNonce() {
        assertFailsWith<IllegalArgumentException> { Preimages.offlineChallenge(ByteArray(0), 1) }
    }

    @Test
    fun negativeWallMsEncodesTwosComplement() {
        val p = Preimages.offlineChallenge(byteArrayOf(1), -1)
        assertTrue(p.copyOfRange(16, 24).all { it == (-1).toByte() })
    }

    @Test
    fun locationCommitmentLayout() {
        val salt = ByteArray(32) { it.toByte() }
        val p = Preimages.locationCommitment(salt, -1, 0x01020304, 500)
        assertEquals(14 + 32 + 12, p.size)
        assertEquals("witness/loc/v1", p.copyOfRange(0, 14).decodeToString())
        assertContentEquals(salt, p.copyOfRange(14, 46))
        assertContentEquals(byteArrayOf(-1, -1, -1, -1), p.copyOfRange(46, 50))
        assertContentEquals(byteArrayOf(1, 2, 3, 4), p.copyOfRange(50, 54))
        assertContentEquals(byteArrayOf(0, 0, 1, 0xf4.toByte()), p.copyOfRange(54, 58))
    }

    @Test
    fun preimagesMatchTheRustVectors() {
        assertEquals(
            "7769746e6573732f6c6f632f763107070707070707070707070707070707070707070707070707070707070707071dcd6500f8a432eb000001f4",
            Preimages.locationCommitment(ByteArray(32) { 7 }, 500_000_000, -123_456_789, 500).toHexString(),
        )
        assertEquals(
            "7769746e6573732d6f66666c696e650102030000018bcfe56800",
            Preimages.offlineChallenge(byteArrayOf(1, 2, 3), 1_700_000_000_000).toHexString(),
        )
    }

    @Test
    fun locationCommitmentValidates() {
        assertFailsWith<IllegalArgumentException> { Preimages.locationCommitment(ByteArray(31), 0, 0, 0) }
        assertFailsWith<IllegalArgumentException> { Preimages.locationCommitment(ByteArray(32), 0, 0, -1) }
    }

    @Test
    fun differentSaltsDiffer() {
        val a = Preimages.locationCommitment(ByteArray(32), 1, 2, 3)
        val b = Preimages.locationCommitment(ByteArray(32) { 1 }, 1, 2, 3)
        assertTrue(!a.contentEquals(b))
    }
}
