package app.witness.nativelib

import app.witness.core.domain.port.OfflineException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.witness_android.MediaErr
import uniffi.witness_android.WitnessException

class NativeException(val code: String, message: String, cause: Throwable? = null) : Exception("$code: $message", cause)

object NativeErrors {
    fun map(e: WitnessException): Exception = when (e) {
        is WitnessException.Network -> OfflineException(e.reason, e)
        is WitnessException.Media -> media(e.err, e)
        is WitnessException.InvalidInput -> NativeException("invalid_input", e.reason, e)
        is WitnessException.Image -> NativeException("image", e.reason, e)
        is WitnessException.Seal -> NativeException("seal", e.reason, e)
        is WitnessException.Signer -> NativeException("signer", e.reason, e)
        is WitnessException.Rejected -> NativeException("rejected", e.reason, e)
        is WitnessException.Malformed -> NativeException("malformed", e.reason, e)
        is WitnessException.WatermarkUnavailable -> NativeException("watermark_unavailable", e.reason, e)
        is WitnessException.Consumed -> NativeException("consumed", "prepared capture already finished", e)
    }

    fun media(err: MediaErr, cause: Throwable): Exception = when (err) {
        is MediaErr.RetryAfterNs -> OfflineException("media busy, retry in ${NativeMapping.nsToMs(err.v1)} ms", cause)
        is MediaErr.Malformed -> NativeException("media.malformed", err.v1, cause)
        MediaErr.Unauthorized -> NativeException("media.unauthorized", "unauthorized", cause)
        MediaErr.NotFound -> NativeException("media.not_found", "not found", cause)
        MediaErr.TooLarge -> NativeException("media.too_large", "too large", cause)
        MediaErr.QuotaExceeded -> NativeException("media.quota_exceeded", "quota exceeded", cause)
        MediaErr.GrantInvalid -> NativeException("media.grant_invalid", "grant invalid", cause)
        MediaErr.BadChunk -> NativeException("media.bad_chunk", "bad chunk", cause)
        MediaErr.HashMismatch -> NativeException("media.hash_mismatch", "hash mismatch", cause)
        MediaErr.Incomplete -> NativeException("media.incomplete", "incomplete", cause)
    }
}

internal object NativeTimeouts {
    const val CALL_MS: Long = 30_000
    const val LONG_MS: Long = 90_000

    fun unreachable(timeoutMs: Long): OfflineException = OfflineException("can't reach the Witness network: no answer within ${timeoutMs / 1_000} s")
}

internal suspend fun <T> remote(timeoutMs: Long = NativeTimeouts.CALL_MS, block: suspend () -> T): T {
    val answer = withTimeoutOrNull(timeoutMs) { Result.success(native { block() }) } ?: throw NativeTimeouts.unreachable(timeoutMs)
    return answer.getOrThrow()
}

internal inline fun <T> native(block: () -> T): T =
    try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: WitnessException) {
        throw NativeErrors.map(e)
    }
