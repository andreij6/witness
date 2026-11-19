package app.witness.core.data

import javax.crypto.AEADBadTagException
import javax.crypto.spec.SecretKeySpec
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse

class AesGcmBlobCipherTest {
    private val cipher = testCipher()

    @Test
    fun roundTrips() {
        val plain = ByteArray(45) { it.toByte() }
        assertContentEquals(plain, cipher.open(cipher.seal(plain)))
    }

    @Test
    fun emptyPlaintextRoundTrips() = assertContentEquals(ByteArray(0), cipher.open(cipher.seal(ByteArray(0))))

    @Test
    fun layoutIsVersionIvCiphertextTag() {
        val sealed = cipher.seal(ByteArray(10))
        assertEquals(1 + 12 + 10 + 16, sealed.size)
        assertEquals(AesGcmBlobCipher.VERSION, sealed[0])
    }

    @Test
    fun ivIsFreshEachTime() {
        assertFalse(cipher.seal(ByteArray(8)).copyOfRange(1, 13).contentEquals(cipher.seal(ByteArray(8)).copyOfRange(1, 13)))
    }

    @Test
    fun tamperingDetected() {
        val sealed = cipher.seal(ByteArray(8) { 3 })
        sealed[sealed.size - 1] = (sealed.last() + 1).toByte()
        assertFailsWith<AEADBadTagException> { cipher.open(sealed) }
    }

    @Test
    fun wrongKeyFails() {
        val other = AesGcmBlobCipher { SecretKeySpec(ByteArray(32) { 8 }, "AES") }
        assertFailsWith<AEADBadTagException> { other.open(cipher.seal(ByteArray(4))) }
    }

    @Test
    fun malformedRejected() {
        assertFailsWith<IllegalArgumentException> { cipher.open(ByteArray(28)) }
        assertFailsWith<IllegalArgumentException> { cipher.open(cipher.seal(ByteArray(1)).also { it[0] = 9 }) }
    }
}
