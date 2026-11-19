package app.witness.di

import app.witness.core.a2ui.TemplateStore
import app.witness.core.domain.port.Crypto
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.test.runTest
import java.util.TimeZone
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class JcaCryptoTest {
    private fun hex(b: ByteArray) = b.joinToString("") { "%02x".format(it) }

    @Test
    fun sha256MatchesKnownVectors() = runTest {
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", hex(JcaCrypto.sha256("abc".encodeToByteArray())))
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", hex(JcaCrypto.sha256(ByteArray(0))))
    }

    @Test
    fun sha256HonoursOffsetAndLength() = runTest {
        val whole = JcaCrypto.sha256("abc".encodeToByteArray())
        assertContentEquals(whole, JcaCrypto.sha256("xxabcyy".encodeToByteArray(), 2, 3))
    }

    @Test
    fun sha256IsRepeatableOnTheSameThread() = runTest {
        val data = ByteArray(1 shl 16) { it.toByte() }
        assertContentEquals(JcaCrypto.sha256(data), JcaCrypto.sha256(data))
    }

    private val key = ByteArray(Crypto.KEY_SIZE) { it.toByte() }
    private val nonce = ByteArray(Crypto.NONCE_SIZE) { (it + 1).toByte() }

    private fun open(sealed: ByteArray, aad: ByteArray, k: ByteArray = key): ByteArray {
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.DECRYPT_MODE, SecretKeySpec(k, "AES"), GCMParameterSpec(128, nonce))
        if (aad.isNotEmpty()) c.updateAAD(aad)
        return c.doFinal(sealed)
    }

    @Test
    fun aesGcmSealRoundTripsAndAppendsTheTag() = runTest {
        val plain = "evidence".encodeToByteArray()
        val aad = "case".encodeToByteArray()
        val sealed = JcaCrypto.aesGcmSeal(key, nonce, aad, plain, 0, plain.size)
        assertEquals(plain.size + Crypto.TAG_SIZE, sealed.size)
        assertContentEquals(plain, open(sealed, aad))
    }

    @Test
    fun aesGcmSealHonoursTheSlice() = runTest {
        val buf = "--evidence--".encodeToByteArray()
        val sealed = JcaCrypto.aesGcmSeal(key, nonce, ByteArray(0), buf, 2, 8)
        assertContentEquals("evidence".encodeToByteArray(), open(sealed, ByteArray(0)))
    }

    @Test
    fun aesGcmSealIsDeterministicForTheSameNonce() = runTest {
        val p = byteArrayOf(1, 2, 3)
        assertContentEquals(JcaCrypto.aesGcmSeal(key, nonce, p, p, 0, 3), JcaCrypto.aesGcmSeal(key, nonce, p, p, 0, 3))
    }

    @Test
    fun aesGcmSealBindsTheAad() = runTest {
        val sealed = JcaCrypto.aesGcmSeal(key, nonce, byteArrayOf(1), byteArrayOf(9), 0, 1)
        assertFailsWith<AEADBadTagException> { open(sealed, byteArrayOf(2)) }
    }

    @Test
    fun aesGcmSealOfEmptyPlaintextIsJustTheTag() = runTest {
        assertEquals(Crypto.TAG_SIZE, JcaCrypto.aesGcmSeal(key, nonce, ByteArray(0), ByteArray(0), 0, 0).size)
    }

    @Test
    fun aesGcmSealRejectsWrongSizes() = runTest {
        assertFailsWith<IllegalArgumentException> { JcaCrypto.aesGcmSeal(ByteArray(16), nonce, ByteArray(0), ByteArray(1), 0, 1) }
        assertFailsWith<IllegalArgumentException> { JcaCrypto.aesGcmSeal(key, ByteArray(8), ByteArray(0), ByteArray(1), 0, 1) }
    }
}

class SecureRandomSourceTest {
    @Test
    fun returnsTheRequestedSize() {
        val r = SecureRandomSource()
        assertEquals(0, r.nextBytes(0).size)
        assertEquals(32, r.nextBytes(32).size)
    }

    @Test
    fun successiveDrawsDiffer() {
        val r = SecureRandomSource()
        assertNotEquals(r.nextBytes(32).toList(), r.nextBytes(32).toList())
    }
}

class LocalTimeTest {
    private val original = TimeZone.getDefault()

    @AfterTest
    fun restore() = TimeZone.setDefault(original)

    @Test
    fun followsTheDefaultZone() {
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
        assertEquals(0, LocalTime.offsetMs(0))
        TimeZone.setDefault(TimeZone.getTimeZone("GMT+05:30"))
        assertEquals(19_800_000, LocalTime.offsetMs(0))
    }

    @Test
    fun honoursDaylightSaving() {
        TimeZone.setDefault(TimeZone.getTimeZone("America/Denver"))
        assertEquals(-7 * 3_600_000L, LocalTime.offsetMs(1_767_225_600_000))
        assertEquals(-6 * 3_600_000L, LocalTime.offsetMs(1_782_864_000_000))
    }

    @Test
    fun systemClockIsWallTime() {
        val before = System.currentTimeMillis()
        val now = SystemClock.nowEpochMillis()
        assertTrue(now >= before && now <= System.currentTimeMillis())
    }
}

class BundledTemplatesTest {
    @Test
    fun loadsOnceEvenUnderConcurrency() = runTest {
        var loads = 0
        val catalog = BundledTemplates { loads++; TemplateStore(emptyList()) }
        (1..16).map { async { catalog.templates() } }.awaitAll()
        assertEquals(1, loads)
    }

    @Test
    fun failedLoadIsRetried() = runTest {
        var loads = 0
        val catalog = BundledTemplates {
            loads++
            if (loads == 1) error("io") else TemplateStore(emptyList())
        }
        assertFailsWith<IllegalStateException> { catalog.templates() }
        assertTrue(catalog.templates().isEmpty())
        assertEquals(2, loads)
    }
}

class AppForegroundTest {
    @Test
    fun countsStartedActivitiesAndNeverGoesNegative() {
        repeat(3) { AppForeground.onStop() }
        assertFalse(AppForeground.isForeground)
        AppForeground.onStart()
        AppForeground.onStart()
        AppForeground.onStop()
        assertTrue(AppForeground.isForeground)
        AppForeground.onStop()
        assertFalse(AppForeground.isForeground)
        AppForeground.onStop()
        assertFalse(AppForeground.isForeground)
    }
}
