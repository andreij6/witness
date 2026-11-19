package app.witness.feature.settings

import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PrincipalTest {
    @Test
    fun crc32MatchesIeeeCheckValue() {
        assertEquals(0xCBF43926.toInt(), Crc32.of("123456789".encodeToByteArray()))
        assertEquals(0, Crc32.of(ByteArray(0)))
    }

    @Test
    fun encodesManagementCanister() = assertEquals("aaaaa-aa", Principal.encode(ByteArray(0)))

    @Test
    fun encodesAnonymousPrincipal() = assertEquals("2vxsx-fae", Principal.encode(byteArrayOf(4)))

    @Test
    fun encodesFirstCanisterId() =
        assertEquals("rrkah-fqaaa-aaaaa-aaaaq-cai", Principal.encode(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 1, 1, 1)))

    @Test
    fun decodesKnownVectors() {
        assertContentEquals(ByteArray(0), Principal.decodeBody("aaaaa-aa"))
        assertContentEquals(byteArrayOf(4), Principal.decodeBody("2vxsx-fae"))
        assertContentEquals(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 1, 1, 1), Principal.decodeBody("rrkah-fqaaa-aaaaa-aaaaq-cai"))
    }

    @Test
    fun roundTripsEveryLength() {
        val random = Random(42)
        for (len in 0..Principal.MAX_BYTES) {
            repeat(8) {
                val bytes = random.nextBytes(len)
                val text = Principal.encode(bytes)
                assertContentEquals(bytes, Principal.decodeBody(text), "len=$len text=$text")
                assertEquals(text, Principal.parse(text)?.text)
            }
        }
    }

    @Test
    fun selfAuthenticatingPrincipalIsMaximalLength() {
        val p = TestPrincipals.alice
        assertEquals(Principal.MAX_TEXT, p.text.length)
        assertTrue(p.isSelfAuthenticating)
        assertEquals(p, Principal.parseSelfAuthenticating(p.text))
    }

    @Test
    fun nonSelfAuthenticatingPrincipalsAreRejectedForLinking() {
        assertNotNull(Principal.parse("2vxsx-fae"))
        assertNull(Principal.parseSelfAuthenticating("2vxsx-fae"))
        assertNull(Principal.parseSelfAuthenticating("aaaaa-aa"))
        assertNull(Principal.parseSelfAuthenticating("rrkah-fqaaa-aaaaa-aaaaq-cai"))
        val wrongTag = Random(3).nextBytes(Principal.MAX_BYTES).also { it[it.lastIndex] = 0x01 }
        assertFalse(Principal.fromBytes(wrongTag).isSelfAuthenticating)
    }

    @Test
    fun rejectsUppercase() = assertNull(Principal.parse(TestPrincipals.alice.text.uppercase()))

    @Test
    fun rejectsChecksumTampering() {
        val text = TestPrincipals.alice.text
        val tampered = text.substring(0, 20) + (if (text[20] == 'a') 'b' else 'a') + text.substring(21)
        assertNull(Principal.parse(tampered))
    }

    @Test
    fun rejectsMissingOrMisplacedDashes() {
        val text = TestPrincipals.alice.text
        assertNull(Principal.parse(text.replace("-", "")))
        assertNull(Principal.parse(text.replaceFirst("-", "") + "-"))
        assertNull(Principal.parse("-$text".take(Principal.MAX_TEXT)))
        assertNull(Principal.parse("aaaa-aaa"))
    }

    @Test
    fun rejectsInvalidCharacters() {
        for (bad in listOf("aaaaa-a1", "aaaaa-a8", "aaaaa-a=", "aaaaa aa", "aaaaa_aa", "ааааа-аа")) assertNull(Principal.parse(bad), bad)
    }

    @Test
    fun rejectsEmptyShortAndOverlong() {
        assertNull(Principal.parse(""))
        assertNull(Principal.parse("a"))
        assertNull(Principal.parse("aaaaa"))
        assertNull(Principal.parse(TestPrincipals.alice.text + "-aa"))
        assertNull(Principal.parse("a".repeat(10_000)))
    }

    @Test
    fun fromBytesRejectsOversizedInput() {
        assertFailsWith<IllegalArgumentException> { Principal.fromBytes(ByteArray(Principal.MAX_BYTES + 1)) }
    }

    @Test
    fun shortFormKeepsEnds() {
        val p = TestPrincipals.alice
        assertEquals(p.text.take(5) + "…" + p.text.takeLast(5), p.short)
        assertEquals("2vxsx-fae", Principal.parse("2vxsx-fae")!!.short)
    }

    @Test
    fun encodingIsDeterministic() {
        val bytes = Random(9).nextBytes(29)
        assertEquals(Principal.encode(bytes), Principal.encode(bytes.copyOf()))
        assertEquals(TestPrincipals.selfAuthenticating(5), TestPrincipals.selfAuthenticating(5))
    }

    @Test
    fun toStringIsText() = assertEquals(TestPrincipals.bob.text, TestPrincipals.bob.toString())
}
