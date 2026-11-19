package app.witness.feature.settings

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IiLinkContractTest {
    private val contract = IiLinkContract()
    private val nonce = "0123456789abcdef0123456789abcdef"
    private val alice = TestPrincipals.alice

    private fun callback(query: String) = "${IiLinkContract.REDIRECT}?$query"

    @Test
    fun requestUrlCarriesVersionNonceAndEncodedRedirect() {
        assertEquals(
            "https://witness.app/ii/link?v=1&nonce=$nonce&redirect=witness%3A%2F%2Fii%2Fcallback",
            contract.requestUrl(nonce),
        )
        assertTrue(IiLinkContract("https://localhost:4943/link").requestUrl(nonce).startsWith("https://localhost:4943/link?v=1&"))
    }

    @Test
    fun nonceIsLowercaseHex() {
        assertEquals("00ff10ab", IiLinkContract.nonceOf(byteArrayOf(0, -1, 16, -85)))
        assertEquals(IiLinkContract.NONCE_BYTES * 2, IiLinkContract.nonceOf(ByteArray(IiLinkContract.NONCE_BYTES)).length)
    }

    @Test
    fun matchesOnlyTheRedirect() {
        assertTrue(IiLinkContract.matches(callback("v=1")))
        assertFalse(IiLinkContract.matches("witness://ii/callbackx?v=1"))
        assertFalse(IiLinkContract.matches("https://ii/callback?v=1"))
        assertFalse(IiLinkContract.matches("witness://ii/callback"))
        assertFalse(IiLinkContract.matches("WITNESS://ii/callback?v=1"))
    }

    @Test
    fun parsesAuthorizedCallback() {
        assertEquals(CallbackResult.Authorized(alice), contract.parseCallback(callback("v=1&nonce=$nonce&principal=${alice.text}"), nonce))
    }

    @Test
    fun parameterOrderDoesNotMatter() {
        assertEquals(CallbackResult.Authorized(alice), contract.parseCallback(callback("principal=${alice.text}&nonce=$nonce&v=1"), nonce))
    }

    @Test
    fun parsesCancellation() {
        assertEquals(CallbackResult.Cancelled, contract.parseCallback(callback("v=1&nonce=$nonce&error=cancelled"), nonce))
    }

    @Test
    fun providerErrorIsReported() {
        assertEquals(CallbackResult.Invalid(LinkError.PROVIDER_ERROR), contract.parseCallback(callback("v=1&nonce=$nonce&error=timeout"), nonce))
    }

    @Test
    fun errorAndPrincipalTogetherIsMalformed() {
        assertEquals(
            CallbackResult.Invalid(LinkError.MALFORMED),
            contract.parseCallback(callback("v=1&nonce=$nonce&error=cancelled&principal=${alice.text}"), nonce),
        )
    }

    @Test
    fun missingPrincipalIsMalformed() {
        assertEquals(CallbackResult.Invalid(LinkError.MALFORMED), contract.parseCallback(callback("v=1&nonce=$nonce"), nonce))
    }

    @Test
    fun missingNonceIsMalformed() {
        assertEquals(CallbackResult.Invalid(LinkError.MALFORMED), contract.parseCallback(callback("v=1&principal=${alice.text}"), nonce))
    }

    @Test
    fun wrongVersionIsRejected() {
        assertEquals(CallbackResult.Invalid(LinkError.UNSUPPORTED_VERSION), contract.parseCallback(callback("v=2&nonce=$nonce&principal=${alice.text}"), nonce))
        assertEquals(CallbackResult.Invalid(LinkError.UNSUPPORTED_VERSION), contract.parseCallback(callback("nonce=$nonce&principal=${alice.text}"), nonce))
    }

    @Test
    fun noPendingRequestIsRejected() {
        assertEquals(CallbackResult.Invalid(LinkError.NO_PENDING_REQUEST), contract.parseCallback(callback("v=1&nonce=$nonce&principal=${alice.text}"), null))
    }

    @Test
    fun nonceMismatchIsRejected() {
        val other = "f".repeat(32)
        assertEquals(CallbackResult.Invalid(LinkError.NONCE_MISMATCH), contract.parseCallback(callback("v=1&nonce=$other&principal=${alice.text}"), nonce))
        assertEquals(CallbackResult.Invalid(LinkError.NONCE_MISMATCH), contract.parseCallback(callback("v=1&nonce=${nonce.dropLast(1)}&principal=${alice.text}"), nonce))
    }

    @Test
    fun invalidOrAnonymousPrincipalIsRejected() {
        assertEquals(CallbackResult.Invalid(LinkError.INVALID_PRINCIPAL), contract.parseCallback(callback("v=1&nonce=$nonce&principal=2vxsx-fae"), nonce))
        assertEquals(CallbackResult.Invalid(LinkError.INVALID_PRINCIPAL), contract.parseCallback(callback("v=1&nonce=$nonce&principal=aaaaa-ab"), nonce))
    }

    @Test
    fun queryRejectsDuplicateKeys() = assertNull(IiLinkContract.queryOf(callback("v=1&v=1")))

    @Test
    fun queryRejectsUnknownKeys() = assertNull(IiLinkContract.queryOf(callback("v=1&redirect=x")))

    @Test
    fun queryRejectsEmptyValuesAndMissingEquals() {
        assertNull(IiLinkContract.queryOf(callback("v=")))
        assertNull(IiLinkContract.queryOf(callback("v")))
        assertNull(IiLinkContract.queryOf(callback("=1")))
        assertNull(IiLinkContract.queryOf(callback("")))
        assertNull(IiLinkContract.queryOf(callback("v=1&")))
        assertNull(IiLinkContract.queryOf(callback("v=1&&nonce=a")))
    }

    @Test
    fun queryRejectsEncodedAndUnsafeCharacters() {
        for (bad in listOf("v=%31", "v=1#frag", "v=1/2", "v=1 ", "v=1=2", "v=ü", "v=A")) assertNull(IiLinkContract.queryOf(callback(bad)), bad)
    }

    @Test
    fun queryRejectsOversizedUri() {
        assertNull(IiLinkContract.queryOf(callback("v=" + "1".repeat(IiLinkContract.MAX_URI))))
    }

    @Test
    fun queryParsesAllKnownKeys() {
        assertEquals(
            mapOf("v" to "1", "nonce" to "ab", "principal" to "x-y", "error" to "e_1"),
            IiLinkContract.queryOf(callback("v=1&nonce=ab&principal=x-y&error=e_1")),
        )
    }

    @Test
    fun malformedUriIsReportedBeforeAnythingElse() {
        assertEquals(CallbackResult.Invalid(LinkError.MALFORMED), contract.parseCallback("https://evil.example/?v=1", nonce))
    }

    @Test
    fun constantTimeEqualsComparesFully() {
        assertTrue(IiLinkContract.constantTimeEquals("abc", "abc"))
        assertTrue(IiLinkContract.constantTimeEquals("", ""))
        assertFalse(IiLinkContract.constantTimeEquals("abc", "abd"))
        assertFalse(IiLinkContract.constantTimeEquals("abc", "ab"))
        assertFalse(IiLinkContract.constantTimeEquals("ab", "abc"))
        assertFalse(IiLinkContract.constantTimeEquals("", "a"))
    }

    @Test
    fun parsingIsDeterministic() {
        val uri = callback("v=1&nonce=$nonce&principal=${alice.text}")
        assertEquals(contract.parseCallback(uri, nonce), contract.parseCallback(uri, nonce))
    }
}
