package app.witness.feature.settings

import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.RandomSource
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

interface IdentityAccount {
    val principal: Flow<Principal?>

    suspend fun link(principal: Principal): Boolean

    suspend fun unlink()
}

enum class LinkError { MALFORMED, UNSUPPORTED_VERSION, NONCE_MISMATCH, INVALID_PRINCIPAL, NO_PENDING_REQUEST, PROVIDER_ERROR, REJECTED, OFFLINE }

sealed interface CallbackResult {
    data class Authorized(val principal: Principal) : CallbackResult

    data object Cancelled : CallbackResult

    data class Invalid(val error: LinkError) : CallbackResult
}

class IiLinkContract(val authorizeUrl: String = AUTHORIZE_URL) {
    fun requestUrl(nonce: String): String = "$authorizeUrl?v=$VERSION&nonce=$nonce&redirect=$REDIRECT_ENCODED"

    fun parseCallback(uri: String, expectedNonce: String?): CallbackResult {
        val params = queryOf(uri) ?: return CallbackResult.Invalid(LinkError.MALFORMED)
        if (params[KEY_VERSION] != VERSION) return CallbackResult.Invalid(LinkError.UNSUPPORTED_VERSION)
        if (expectedNonce == null) return CallbackResult.Invalid(LinkError.NO_PENDING_REQUEST)
        val nonce = params[KEY_NONCE] ?: return CallbackResult.Invalid(LinkError.MALFORMED)
        if (!constantTimeEquals(nonce, expectedNonce)) return CallbackResult.Invalid(LinkError.NONCE_MISMATCH)
        val error = params[KEY_ERROR]
        val principal = params[KEY_PRINCIPAL]
        return when {
            error != null && principal != null -> CallbackResult.Invalid(LinkError.MALFORMED)
            error == ERROR_CANCELLED -> CallbackResult.Cancelled
            error != null -> CallbackResult.Invalid(LinkError.PROVIDER_ERROR)
            principal == null -> CallbackResult.Invalid(LinkError.MALFORMED)
            else -> Principal.parseSelfAuthenticating(principal)?.let(CallbackResult::Authorized) ?: CallbackResult.Invalid(LinkError.INVALID_PRINCIPAL)
        }
    }

    companion object {
        const val VERSION = "1"
        const val AUTHORIZE_URL = "https://witness.app/ii/link"
        const val REDIRECT = "witness://ii/callback"
        const val REDIRECT_ENCODED = "witness%3A%2F%2Fii%2Fcallback"
        const val MAX_URI = 512
        const val NONCE_BYTES = 16
        const val ERROR_CANCELLED = "cancelled"
        private const val KEY_VERSION = "v"
        private const val KEY_NONCE = "nonce"
        private const val KEY_PRINCIPAL = "principal"
        private const val KEY_ERROR = "error"
        private val KEYS = setOf(KEY_VERSION, KEY_NONCE, KEY_PRINCIPAL, KEY_ERROR)
        private const val PREFIX = "$REDIRECT?"

        fun matches(uri: String): Boolean = uri.startsWith(PREFIX)

        fun queryOf(uri: String): Map<String, String>? {
            if (uri.length > MAX_URI || !matches(uri)) return null
            val out = HashMap<String, String>(KEYS.size)
            var start = PREFIX.length
            while (start <= uri.length) {
                val end = uri.indexOf('&', start).let { if (it < 0) uri.length else it }
                val eq = uri.indexOf('=', start)
                if (eq < 0 || eq >= end) return null
                val key = uri.substring(start, eq)
                val value = uri.substring(eq + 1, end)
                if (key !in KEYS || value.isEmpty() || !value.all(::isSafe) || out.put(key, value) != null) return null
                start = end + 1
            }
            return out
        }

        fun nonceOf(bytes: ByteArray): String = bytes.toHexString()

        fun constantTimeEquals(a: String, b: String): Boolean {
            var diff = a.length xor b.length
            for (i in 0 until minOf(a.length, b.length)) diff = diff or (a[i].code xor b[i].code)
            return diff == 0
        }

        private fun isSafe(c: Char): Boolean = c in 'a'..'z' || c in '0'..'9' || c == '-' || c == '_'
    }
}

sealed interface LinkStatus {
    data object Idle : LinkStatus

    data object Waiting : LinkStatus

    data object Linking : LinkStatus

    data class Linked(val principal: Principal) : LinkStatus

    data class Failed(val error: LinkError) : LinkStatus
}

class IdentityLinkFlow(
    private val account: IdentityAccount,
    private val random: RandomSource,
    private val contract: IiLinkContract = IiLinkContract(),
) {
    private val mutableStatus = MutableStateFlow<LinkStatus>(LinkStatus.Idle)
    private var pendingNonce: String? = null

    val status: StateFlow<LinkStatus> = mutableStatus.asStateFlow()

    fun begin(): String {
        val nonce = IiLinkContract.nonceOf(random.nextBytes(IiLinkContract.NONCE_BYTES))
        pendingNonce = nonce
        mutableStatus.value = LinkStatus.Waiting
        return contract.requestUrl(nonce)
    }

    fun reset() {
        pendingNonce = null
        mutableStatus.value = LinkStatus.Idle
    }

    suspend fun complete(uri: String): LinkStatus {
        val result = contract.parseCallback(uri, pendingNonce)
        if (result is CallbackResult.Invalid && result.error == LinkError.NONCE_MISMATCH) return publish(LinkStatus.Failed(result.error))
        pendingNonce = null
        return publish(
            when (result) {
                is CallbackResult.Authorized -> link(result.principal)
                CallbackResult.Cancelled -> LinkStatus.Idle
                is CallbackResult.Invalid -> LinkStatus.Failed(result.error)
            },
        )
    }

    private suspend fun link(principal: Principal): LinkStatus {
        mutableStatus.value = LinkStatus.Linking
        return try {
            if (account.link(principal)) LinkStatus.Linked(principal) else LinkStatus.Failed(LinkError.REJECTED)
        } catch (_: OfflineException) {
            LinkStatus.Failed(LinkError.OFFLINE)
        }
    }

    private fun publish(status: LinkStatus): LinkStatus = status.also { mutableStatus.value = it }
}
