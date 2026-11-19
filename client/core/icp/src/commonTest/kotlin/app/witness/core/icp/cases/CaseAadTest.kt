package app.witness.core.icp.cases

import app.witness.core.icp.hex
import app.witness.core.icp.id.CaseId
import app.witness.core.icp.toHex
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class CaseAadTest {
    private val caseId = CaseId.of(ByteArray(16) { 0xab.toByte() })
    private val emptySha = hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")

    @Test
    fun layoutMatchesArchitectureVector() {
        val aad = CaseAad.build(caseId, 2, 5, emptySha)
        assertEquals(71, aad.size)
        assertEquals(CaseAad.SIZE, aad.size)
        assertEquals(
            "7769746e6573732f636173652f7631" + "ab".repeat(16) + "00000002" + "00000005" +
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            aad.toHex(),
        )
    }

    @Test
    fun indexAndCountAreBigEndian() {
        val aad = CaseAad.build(caseId, 0x01020304, 0x7fffffff, emptySha)
        assertContentEquals(hex("01020304 7fffffff"), aad.copyOfRange(31, 39))
    }

    @Test
    fun everyIndexProducesDistinctAad() {
        val seen = (0 until 64).map { CaseAad.build(caseId, it, 64, emptySha).toHex() }.toSet()
        assertEquals(64, seen.size)
    }

    @Test
    fun rejectsInvalidArguments() {
        assertFailsWith<IllegalArgumentException> { CaseAad.build(caseId, 0, 0, emptySha) }
        assertFailsWith<IllegalArgumentException> { CaseAad.build(caseId, 5, 5, emptySha) }
        assertFailsWith<IllegalArgumentException> { CaseAad.build(caseId, -1, 5, emptySha) }
        assertFailsWith<IllegalArgumentException> { CaseAad.build(caseId, 0, 1, ByteArray(31)) }
    }
}
