package app.witness.nativelib

import app.witness.core.domain.port.OfflineException
import kotlinx.coroutines.CancellationException
import uniffi.witness_android.MediaErr
import uniffi.witness_android.WitnessException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertSame

class NativeErrorsTest {
    private fun code(e: WitnessException) = assertIs<NativeException>(NativeErrors.map(e)).code

    @Test
    fun networkFailuresAreOffline() {
        val e = WitnessException.Network("connection refused")
        val mapped = assertIs<OfflineException>(NativeErrors.map(e))
        assertEquals("connection refused", mapped.message)
        assertSame(e, mapped.cause)
    }

    @Test
    fun everyOtherVariantKeepsItsCodeReasonAndCause() {
        val cases = listOf(
            WitnessException.InvalidInput("a") to "invalid_input",
            WitnessException.Image("b") to "image",
            WitnessException.Seal("c") to "seal",
            WitnessException.Signer("d") to "signer",
            WitnessException.Rejected("e") to "rejected",
            WitnessException.Malformed("f") to "malformed",
            WitnessException.WatermarkUnavailable("g") to "watermark_unavailable",
            WitnessException.Consumed() to "consumed",
        )
        for ((e, expected) in cases) {
            val mapped = assertIs<NativeException>(NativeErrors.map(e))
            assertEquals(expected, mapped.code)
            assertSame(e, mapped.cause)
        }
        assertEquals("rejected: quota: retry after 5 ns", NativeErrors.map(WitnessException.Rejected("quota: retry after 5 ns")).message)
    }

    @Test
    fun everyMediaErrorHasItsOwnCode() {
        val cases = listOf(
            MediaErr.Malformed("x") to "media.malformed",
            MediaErr.Unauthorized to "media.unauthorized",
            MediaErr.NotFound to "media.not_found",
            MediaErr.TooLarge to "media.too_large",
            MediaErr.QuotaExceeded to "media.quota_exceeded",
            MediaErr.GrantInvalid to "media.grant_invalid",
            MediaErr.BadChunk to "media.bad_chunk",
            MediaErr.HashMismatch to "media.hash_mismatch",
            MediaErr.Incomplete to "media.incomplete",
        )
        for ((err, expected) in cases) assertEquals(expected, code(WitnessException.Media(err)))
        assertEquals(cases.size + 1, variantsOf(MediaErr::class.java))
    }

    @Test
    fun mediaRetryAfterIsOffline() {
        val mapped = NativeErrors.map(WitnessException.Media(MediaErr.RetryAfterNs(3_000_000uL)))
        assertIs<OfflineException>(mapped)
        assertEquals("media busy, retry in 3 ms", mapped.message)
    }

    @Test
    fun everyWitnessExceptionVariantIsCovered() {
        assertEquals(10, variantsOf(WitnessException::class.java))
    }

    @Test
    fun nativeWrapperTranslatesAndPassesThrough() {
        assertFailsWith<OfflineException> { native { throw WitnessException.Network("down") } }
        assertFailsWith<IllegalStateException> { native { error("other") } }
        assertFailsWith<CancellationException> { native { throw CancellationException("stop") } }
        assertEquals(4, native { 4 })
    }
}
