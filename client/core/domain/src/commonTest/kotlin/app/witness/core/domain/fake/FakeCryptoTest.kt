package app.witness.core.domain.fake

import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class FakeCryptoTest {
    private fun hex(b: ByteArray) = b.joinToString("") { (it.toInt() and 0xff).toString(16).padStart(2, '0') }

    @Test
    fun sha256KnownVectors() {
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", hex(SoftwareSha256.digest(ByteArray(0))))
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", hex(SoftwareSha256.digest("abc".encodeToByteArray())))
        assertEquals(
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            hex(SoftwareSha256.digest("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".encodeToByteArray())),
        )
        assertEquals("41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3", hex(SoftwareSha256.digest(ByteArray(1000) { 'a'.code.toByte() })))
    }

    @Test
    fun sha256PaddingBoundaries() {
        assertEquals("463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59", hex(SoftwareSha256.digest(ByteArray(55) { it.toByte() })))
        assertEquals("fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108", hex(SoftwareSha256.digest(ByteArray(64) { it.toByte() })))
    }

    @Test
    fun sha256Ranges() {
        val data = ByteArray(10) + "abc".encodeToByteArray() + ByteArray(5)
        assertContentEquals(SoftwareSha256.digest("abc".encodeToByteArray()), SoftwareSha256.digest(data, 10, 3))
        assertFailsWith<IllegalArgumentException> { SoftwareSha256.digest(data, 17, 2) }
        assertFailsWith<IllegalArgumentException> { SoftwareSha256.digest(data, -1, 2) }
    }

    @Test
    fun sealOpenRoundTripAndTamper() = runTest {
        val c = FakeCrypto()
        val key = ByteArray(32) { 1 }
        val nonce = ByteArray(12) { 2 }
        val aad = "aad".encodeToByteArray()
        val pt = ByteArray(100) { it.toByte() }
        val sealed = c.aesGcmSeal(key, nonce, aad, pt, 10, 70)
        assertEquals(86, sealed.size)
        assertContentEquals(pt.copyOfRange(10, 80), assertNotNull(c.open(key, nonce, aad, sealed)))
        assertNull(c.open(key, nonce, "aae".encodeToByteArray(), sealed))
        assertNull(c.open(ByteArray(32), nonce, aad, sealed))
        assertNull(c.open(key, ByteArray(12), aad, sealed))
        assertNull(c.open(key, nonce, aad, sealed.copyOf().also { it[0] = (it[0] + 1).toByte() }))
        assertNull(c.open(key, nonce, aad, ByteArray(3)))
        assertFailsWith<IllegalArgumentException> { c.aesGcmSeal(ByteArray(16), nonce, aad, pt, 0, 1) }
    }
}
