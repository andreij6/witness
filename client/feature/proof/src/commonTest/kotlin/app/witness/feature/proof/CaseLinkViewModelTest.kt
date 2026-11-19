package app.witness.feature.proof

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeVerifyEngine
import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.CheckResult
import app.witness.core.model.LocationConfidence
import app.witness.core.model.Platform
import app.witness.core.model.SummarySource
import app.witness.core.model.Verdict
import app.witness.core.ui.Proves
import app.witness.core.model.EnumBits
import app.witness.core.model.VerdictFlag
import app.witness.core.model.SecurityLevel
import app.witness.core.designsystem.WitnessState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.yield
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class CaseLinkViewModelTest {
    private val clock = FakeClock(Fixtures.T0)
    private val registry = FakeRegistry(clock)
    private val index = FakeIndex()
    private val media = FakeMedia()
    private val check = CheckImage(FakeVerifyEngine(), registry, index, media, Platform.WEB)
    private val caseId = CaseId.of(1, ByteArray(15).also { it.fill(4) })

    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private fun vm(result: CaseLinkResult, id: String = caseId.text, fragment: String = "k=abc", checker: CheckImage = check, parallelism: Int = 4) =
        CaseLinkViewModel(id, fragment, { _, _ -> result }, checker, TimeText.UTC, parallelism)

    private fun item(vararg b: Byte, label: String = "Intersection") = OpenedItem(label, ByteString.of(b))

    @Test
    fun badIdMissingKeyAndOpenerOutcomes() {
        assertEquals(CaseLinkState.NotFound, vm(CaseLinkResult.NotFound, id = "nope").state.value)
        assertEquals(CaseLinkState.MissingKey, vm(CaseLinkResult.NotFound, fragment = " ").state.value)
        assertEquals(CaseLinkState.NotFound, vm(CaseLinkResult.NotFound).state.value)
        assertEquals(CaseLinkState.Tampered, vm(CaseLinkResult.Tampered).state.value)
        assertEquals(CaseLinkState.Expired("Nov 18"), vm(CaseLinkResult.Expired(Fixtures.T0)).state.value)
        assertEquals(CaseLinkState.Expired(null), vm(CaseLinkResult.Expired(null)).state.value)
    }

    @Test
    fun openerFailureIsOffline() {
        val vm = CaseLinkViewModel(caseId.text, "k=x", { _, _ -> error("down") }, check)
        assertEquals(CaseLinkState.Offline, vm.state.value)
    }

    @Test
    fun openedCaseVerifiesEveryItem() = runTest {
        val a = item(1)
        val rec = registry.putRecord(Fixtures.recordId, Fixtures.claim())
        index.addExact(0, ByteString.of(SoftwareSha256.digest(a.jpeg.toByteArray())), rec.recordIx)
        val case = OpenedCase("Fender bender", "A summary", Fixtures.T0, Fixtures.T0 + 7 * 86_400_000, listOf(a, item(2, label = "")))
        val s = assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(case)).state.value)
        assertEquals("Fender bender", s.title)
        assertEquals("Shared Nov 18 · 2 items · expires Nov 25", s.meta)
        assertEquals("Nov 25, 2026", s.expires)
        assertEquals("A summary", s.summary)
        assertEquals(listOf("Photo 1 · Intersection", "Photo 2"), s.items.map { it.label })
        assertEquals(WitnessState.Original, s.items[0].state)
        assertEquals("2:12:00 PM · location high", s.items[0].meta)
        assertEquals(Fixtures.recordId, s.items[0].recordId)
        assertEquals(WitnessState.NoRecord, s.items[1].state)
        assertEquals("1 of 2 verified", s.verdict!!.headline)
        assertEquals(WitnessState.NoRecord, s.verdict.state)
        assertFalse(s.dev)
    }

    @Test
    fun summaryWithoutProvenanceIsNotCalledAiDrafted() = runTest {
        val case = OpenedCase("Leak", "Water under the sink", Fixtures.T0, null, listOf(item(1)))
        val s = assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(case)).state.value)
        assertEquals(null, s.summarySource)
        assertEquals("Sender's summary", SenderSummaryCopy.label(s.summarySource))
        assertFalse("AI" in SenderSummaryCopy.label(s.summarySource) || "AI" in SenderSummaryCopy.footnote(s.summarySource))
    }

    @Test
    fun summaryProvenanceReachesThePage() = runTest {
        for (source in SummarySource.entries) {
            val case = OpenedCase("Leak", "Water", Fixtures.T0, null, listOf(item(1)), source)
            assertEquals(source, assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(case)).state.value).summarySource)
        }
        val noSummary = OpenedCase("Leak", null, Fixtures.T0, null, listOf(item(1)), SummarySource.GENERATED)
        assertEquals(null, assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(noSummary)).state.value).summarySource)
    }

    @Test
    fun onlyGeneratedSummariesAreLabelledAiDrafted() {
        assertEquals("Sender's summary · AI-drafted", SenderSummaryCopy.label(SummarySource.GENERATED))
        assertTrue(SenderSummaryCopy.isGenerated(SummarySource.GENERATED))
        assertEquals("Sender's summary · written by the sender", SenderSummaryCopy.label(SummarySource.WRITTEN))
        assertEquals("Sender's summary · edited by the sender", SenderSummaryCopy.label(SummarySource.EDITED))
        listOf(SummarySource.EDITED, SummarySource.WRITTEN, null).forEach {
            assertFalse("AI-drafted" in SenderSummaryCopy.label(it))
            assertFalse(SenderSummaryCopy.isGenerated(it))
        }
    }

    @Test
    fun devItemsMarkTheWholeCaseAsDev() = runTest {
        val a = item(1)
        val rec = registry.putRecord(Fixtures.recordId, Fixtures.claim(security = SecurityLevel.DEV))
        index.addExact(0, ByteString.of(SoftwareSha256.digest(a.jpeg.toByteArray())), rec.recordIx)
        val case = OpenedCase("Dev case", null, Fixtures.T0, null, listOf(a, item(2, label = "")))
        assertTrue(assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(case)).state.value).dev)
        assertTrue(CaseLinkViewModel.isDev(CheckResult(Verdict.ORIGINAL, flags = EnumBits.none<VerdictFlag>() + VerdictFlag.DEV_ATTESTATION)))
        assertFalse(CaseLinkViewModel.isDev(CheckResult(Verdict.ORIGINAL)))
        assertTrue(Proves.CaseLinkDev.none { it.proven && it.text.contains("Genuine") })
    }

    @Test
    fun parallelismIsBounded() = runTest {
        var running = 0
        var peak = 0
        val engine = VerifyEngine { _: ByteString, _: VerifyOracle, _: JudgeOptions, _: StageSink ->
            running++
            peak = maxOf(peak, running)
            yield()
            running--
            CheckResult.NO_RECORD
        }
        val checker = CheckImage(engine, registry, index, media, Platform.WEB)
        val case = OpenedCase("c", null, 0, null, List(9) { item(it.toByte()) })
        val s = assertIs<CaseLinkState.Open>(vm(CaseLinkResult.Opened(case), checker = checker, parallelism = 2).state.value)
        assertEquals(9, s.items.size)
        assertTrue(peak in 1..2, "peak $peak")
    }

    @Test
    fun summarizeCountsEachVerdict() {
        val blur = CheckResult(Verdict.MATCHES_WITH_CHANGES, declared = listOf(Fixtures.blur(BlurClass.FACE)))
        val all = CaseLinkViewModel.summarize(listOf(CheckResult(Verdict.ORIGINAL), CheckResult(Verdict.ORIGINAL), CheckResult(Verdict.ORIGINAL), blur))
        assertEquals("4 of 4 verified", all.headline)
        assertEquals("3 original · 1 with a declared edit (face blurred).", all.detail)
        assertEquals(WitnessState.Original, all.state)
        val bad = CaseLinkViewModel.summarize(listOf(CheckResult(Verdict.ALTERED), CheckResult.NO_RECORD, blur, blur))
        assertEquals("2 of 4 verified", bad.headline)
        assertEquals("2 with declared edits (face blurred) · 1 altered · 1 without a record.", bad.detail)
        assertEquals(WitnessState.Altered, bad.state)
        assertEquals(WitnessState.NoRecord, CaseLinkViewModel.summarize(emptyList()).state)
    }

    @Test
    fun itemUiMarksBlurredEdits() {
        val r = CheckResult(Verdict.MATCHES_WITH_CHANGES, declared = listOf(Fixtures.blur(BlurClass.FACE)), reference = null)
        val ui = CaseLinkViewModel.itemUi(3, item(1, label = "Wide shot"), r, TimeText.UTC)
        assertEquals("Photo 4 · Wide shot", ui.label)
        assertEquals(WitnessState.Edited, ui.state)
        assertEquals("Face blurred", ui.stateText)
        assertEquals("face blurred", ui.meta)
    }

    @Test
    fun phrasesAndWords() {
        assertEquals("face and plate blurred", CaseLinkViewModel.blurPhrase(listOf(Fixtures.blur(), Fixtures.blur(BlurClass.FACE), Fixtures.blur())))
        assertEquals("face, plate and house number blurred", CaseLinkViewModel.blurPhrase(listOf(Fixtures.blur(BlurClass.HOUSE_NO), Fixtures.blur(), Fixtures.blur(BlurClass.FACE))))
        assertEquals(listOf("high", "medium", "low"), LocationConfidence.entries.map(CaseLinkViewModel::confidenceWord))
        assertEquals("Photo 1", CaseLinkViewModel.itemLabel(0, "  "))
    }
}
