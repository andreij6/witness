package app.witness.web

import app.witness.core.domain.Clock
import app.witness.core.icp.witness.BridgeCases
import app.witness.core.icp.witness.OpenedBundle
import app.witness.core.icp.witness.StageListener
import app.witness.core.icp.witness.WitnessBridge
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.CheckStage
import app.witness.core.model.RecordId
import app.witness.core.model.StageState
import app.witness.core.model.Verdict
import app.witness.feature.proof.CaseLinkResult
import app.witness.feature.proof.ProofPageState
import app.witness.feature.proof.ProofPageViewModel
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WebGraphTest {
    private val recordId = "WERC8GMRZGE196QV"
    private val keyHex = "00112233445566778899aabbccddeeff"
    private val claim = """{"assetHash":"${"aa".repeat(32)}","captureId":"5","coarse":null,"counter":"1","height":480,"keyId":"$keyHex",
        "kind":{"type":"original"},"pixelHash":"${"bb".repeat(32)}","wallMs":"1700000000000","width":640,"gnssMs":"1700000000100",
        "location":{"commit":null,"confidence":0,"reasons":1},"checks":{"keySecurity":2,"boot":0,"deviceLocked":true,"mockLocation":false,"gnss":1,"recapture":0}}"""
    private val verdict = """{"declared":[],"flags":32,"outcome":"ORIGINAL","reasonBits":1,"recordId":"$recordId","regions":[],"version":1,
        "reference":{"recordId":"$recordId","claim":$claim,"anchor":{"recordIx":"0","anchoredNs":"1700000002000000000"},"evidence":"seal",
        "confidenceQ16":65536,"comparedQ16":65536,"changedQ16":0}}"""
    private val key = """{"keyId":"$keyHex","pubkey":"04","security":"Dev","provisioning":"Dev","freshness":"Fresh","patch":0,
        "enrolledNs":"1000000","activeUntilNs":"9000000000000000000","history":[{"status":"Active","sinceNs":"1000000"}],"chainSerials":[]}"""

    private inner class Bridge(var opened: OpenedBundle = OpenedBundle(OpenedBundle.NOT_FOUND, null, null), var offline: Boolean = false) : WitnessBridge {
        override suspend fun check(input: ByteArray, listener: StageListener): String {
            listener.onStage(0, 1, 0, 0)
            listener.onStage(2, 1, 1, 1)
            return verdict
        }

        override suspend fun proof(recordId: String): String? {
            if (offline) throw IllegalStateException("offline")
            return if (recordId == this@WebGraphTest.recordId) """{"recordIx":"0","recordId":"$recordId","anchoredNs":"1700000002000000000","claim":$claim}""" else null
        }

        override suspend fun key(keyIdHex: String): String? = key.takeIf { keyIdHex == keyHex }

        override suspend fun preview(recordId: String): ByteArray? = byteArrayOf(-1, -40).takeIf { recordId == this@WebGraphTest.recordId }

        override suspend fun openCase(caseId: String, fragment: String): OpenedBundle = opened
    }

    private val clock = Clock { 42L }

    @Test
    fun checkRunsThroughTheBridgeEngine() = runTest {
        val graph = WebGraph(clock, Bridge())
        val progress = graph.checkImage(ByteString.of(byteArrayOf(1, 2, 3))).toList()
        val last = progress.last()
        assertEquals(Verdict.ORIGINAL, last.result?.verdict)
        assertEquals(recordId, last.result?.reference?.recordId?.text)
        assertEquals(StageState.DONE, last[CheckStage.FINGERPRINTS].state)
        assertEquals(StageState.SKIPPED, last[CheckStage.WATERMARK].state)
        assertTrue(progress.any { it[CheckStage.FINGERPRINTS].state == StageState.RUNNING && it[CheckStage.FINGERPRINTS].total == 1 })
    }

    @Test
    fun proofPageLoadsVerifiedRecordKeyAndPreview() = runTest {
        val graph = WebGraph(clock, Bridge())
        val state = ProofPageViewModel(recordId, graph.registry, graph.media).load()
        val page = assertIs<ProofPageState.Found>(state).page
        assertEquals(Verdict.ORIGINAL, page.verdict)
        assertEquals(2, page.image.size)
        assertTrue(page.warning.orEmpty().contains("DEV"))
    }

    @Test
    fun proofPageStates() = runTest {
        val graph = WebGraph(clock, Bridge())
        assertEquals(ProofPageState.NotFound, ProofPageViewModel("0123456789ABCDEF", graph.registry, graph.media).load())
        assertEquals(ProofPageState.NotFound, ProofPageViewModel("not-an-id", graph.registry, graph.media).load())
        val offline = WebGraph(clock, Bridge(offline = true))
        assertEquals(ProofPageState.Offline, ProofPageViewModel(recordId, offline.registry, offline.media).load())
    }

    @Test
    fun caseLinksMapEveryOutcome() = runTest {
        val caseId = CaseId.of(0, ByteArray(CaseId.RANDOM_SIZE))
        val bridge = Bridge()
        val links = WebCaseLinks(BridgeCases(bridge), clock)
        assertEquals(CaseLinkResult.NotFound, links.open(caseId, "#k=x"))
        bridge.opened = OpenedBundle(OpenedBundle.TAMPERED, null, null)
        assertEquals(CaseLinkResult.Tampered, links.open(caseId, "#k=x"))
        val items = """[{"recordId":"$recordId","offset":0,"len":2,"wallMs":"1700000000000"},{"recordId":"$recordId","offset":1,"len":1,"wallMs":"1700000009000"}]"""
        bridge.opened = OpenedBundle(OpenedBundle.OPENED, """{"title":"T","summary":null,"items":$items}""", byteArrayOf(-1, -40))
        val case = assertIs<CaseLinkResult.Opened>(links.open(caseId, "#k=x")).case
        assertEquals("T", case.title)
        assertNull(case.summary)
        assertNull(case.expiresAtMs)
        assertEquals(1_700_000_009_000L, case.sharedAtMs)
        assertEquals(RecordId.parse(recordId), case.items[0].recordId)
        assertEquals(2, case.items[0].jpeg.size)
        bridge.opened = OpenedBundle(OpenedBundle.OPENED, """{"title":"T","summary":"S","items":[{"recordId":"$recordId","offset":0,"len":2,"wallMs":null}]}""", byteArrayOf(1, 2))
        assertEquals(42L, assertIs<CaseLinkResult.Opened>(links.open(caseId, "#k=x")).case.sharedAtMs)
        bridge.opened = OpenedBundle(OpenedBundle.OPENED, """{"title":"T","summary":null,"items":$items,"expiresMs":"1702592000000"}""", byteArrayOf(-1, -40))
        assertEquals(1_702_592_000_000L, assertIs<CaseLinkResult.Opened>(links.open(caseId, "#k=x")).case.expiresAtMs)
    }
}
