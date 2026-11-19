package app.witness.core.icp.witness

import app.witness.core.icp.witness.WitnessFixtures.RECORD_ID
import app.witness.core.model.CaseId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class BridgeCasesTest {
    private val caseId = CaseId.of(0, ByteArray(CaseId.RANDOM_SIZE) { it.toByte() })
    private val bundle = ByteArray(10) { (it + 1).toByte() }

    private fun opened(json: String?, bytes: ByteArray? = bundle) = OpenedBundle(OpenedBundle.OPENED, json, bytes)

    private fun items(vararg item: String) = """{"title":"Fender bender","summary":"AI text","items":[${item.joinToString(",")}]}"""

    @Test
    fun openedCaseSlicesJpegsOutOfTheBundle() = runTest {
        val bridge = FakeBridge(opened = opened(items("""{"recordId":"$RECORD_ID","offset":2,"len":3,"wallMs":"1700000000000"}""", """{"recordId":"bad","offset":0,"len":10,"wallMs":null}""")))
        val result = assertIs<CaseOpening.Opened>(BridgeCases(bridge).open(caseId, "#k=x"))
        assertEquals("Fender bender", result.title)
        assertEquals("AI text", result.summary)
        assertEquals(listOf<Byte>(3, 4, 5), result.items[0].jpeg.toByteArray().toList())
        assertEquals(RECORD_ID, result.items[0].recordId?.text)
        assertEquals(1_700_000_000_000L, result.items[0].capturedAtMs)
        assertNull(result.items[1].recordId)
        assertNull(result.items[1].capturedAtMs)
        assertEquals(listOf("case:${caseId.text}"), bridge.calls)
        assertNull(result.expiresAtMs)
    }

    @Test
    fun theCanisterExpiryTravelsWithTheOpenedCase() = runTest {
        val item = """{"recordId":"$RECORD_ID","offset":0,"len":2,"wallMs":null}"""
        val withExpiry = """{"title":"T","summary":null,"items":[$item],"expiresMs":"1702592000000"}"""
        assertEquals(1_702_592_000_000L, assertIs<CaseOpening.Opened>(BridgeCases(FakeBridge(opened = opened(withExpiry))).open(caseId, "#k=x")).expiresAtMs)
        val unknown = """{"title":"T","summary":null,"items":[$item],"expiresMs":null}"""
        assertNull(assertIs<CaseOpening.Opened>(BridgeCases(FakeBridge(opened = opened(unknown))).open(caseId, "#k=x")).expiresAtMs)
    }

    @Test
    fun statusesMapToOutcomes() = runTest {
        assertEquals(CaseOpening.NotFound, BridgeCases(FakeBridge()).open(caseId, "#k=x"))
        assertEquals(CaseOpening.Tampered, BridgeCases(FakeBridge(opened = OpenedBundle(OpenedBundle.TAMPERED, null, null))).open(caseId, "#k=x"))
        assertEquals(CaseOpening.Tampered, BridgeCases(FakeBridge(opened = OpenedBundle("weird", null, null))).open(caseId, "#k=x"))
    }

    @Test
    fun inconsistentBundlesAreTreatedAsTampered() = runTest {
        val cases = listOf(
            opened(null),
            opened(items(), null),
            opened(items()),
            opened(items("""{"recordId":"$RECORD_ID","offset":8,"len":3}""")),
            opened(items("""{"recordId":"$RECORD_ID","offset":-1,"len":3}""")),
            opened(items("""{"recordId":"$RECORD_ID","offset":0,"len":0}""")),
            opened(items("""{"recordId":"$RECORD_ID","offset":2147483647,"len":2147483647}""")),
        )
        for (c in cases) assertEquals(CaseOpening.Tampered, BridgeCases(FakeBridge(opened = c)).open(caseId, "#k=x"))
    }
}
