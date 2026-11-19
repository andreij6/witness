package app.witness.core.platform

import app.witness.core.platform.crypto.EcdsaSignature
import java.math.BigInteger
import java.security.KeyPairGenerator
import java.security.Signature
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EcdsaInteropTest {
    private val pair = KeyPairGenerator.getInstance("EC").apply { initialize(ECGenParameterSpec("secp256r1")) }.generateKeyPair()

    @Test
    fun jcaDerSignaturesConvertToVerifiableRaw() {
        val random = Random(7)
        repeat(200) {
            val msg = random.nextBytes(random.nextInt(0, 256))
            val der = Signature.getInstance("SHA256withECDSA").run { initSign(pair.private); update(msg); sign() }
            val raw = EcdsaSignature.derToRaw(der)
            assertTrue(EcdsaSignature.isLowS(raw))
            val ok = Signature.getInstance("SHA256withECDSAinP1363Format").run { initVerify(pair.public); update(msg); verify(raw) }
            assertTrue(ok)
        }
    }

    @Test
    fun halfOrderMatchesCurve() {
        val n = (pair.public as ECPublicKey).params.order
        val half = n.shiftRight(1)
        val raw = ByteArray(64).also { it[31] = 1 }
        val bytes = half.toByteArray().let { if (it.size > 32) it.copyOfRange(it.size - 32, it.size) else it }
        bytes.copyInto(raw, 64 - bytes.size)
        assertTrue(EcdsaSignature.isLowS(raw))
        val above = half.add(BigInteger.ONE).toByteArray().let { it.copyOfRange(it.size - 32, it.size) }
        above.copyInto(raw, 32)
        assertTrue(!EcdsaSignature.isLowS(raw))
        val normalized = EcdsaSignature.derToRaw(EcdsaSignature.rawToDer(raw))
        val expected = n.subtract(half.add(BigInteger.ONE)).toByteArray().let { if (it.size > 32) it.copyOfRange(it.size - 32, it.size) else it }
        assertContentEquals(ByteArray(32 - expected.size) + expected, normalized.copyOfRange(32, 64))
        assertEquals(64, normalized.size)
    }
}
