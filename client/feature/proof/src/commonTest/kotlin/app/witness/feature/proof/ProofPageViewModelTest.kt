package app.witness.feature.proof

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.ClaimKind
import app.witness.core.model.Edit
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyStatus
import app.witness.core.model.Provisioning
import app.witness.core.model.SecurityLevel
import app.witness.core.model.StatusChange
import app.witness.core.model.Verdict
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import app.witness.core.ui.Proves
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ProofPageViewModelTest {
    private val clock = FakeClock(Fixtures.T0)
    private val registry = FakeRegistry(clock)
    private val media = FakeMedia()
    private val image = ByteString.of(byteArrayOf(1, 2, 3))

    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private fun vm(id: String = Fixtures.recordId.text) = ProofPageViewModel(id, registry, media)

    private val blurred = ClaimKind.Derivative(Fixtures.otherId, Fixtures.captureId, listOf(Fixtures.blur(BlurClass.HOUSE_NO), Fixtures.blur()))

    @Test
    fun malformedOrUnknownIdsAreNotFound() {
        assertEquals(ProofPageState.NotFound, vm("garbage").state.value)
        assertEquals(ProofPageState.NotFound, vm().state.value)
    }

    @Test
    fun recordWithReferenceIsFound() {
        registry.putRecord(Fixtures.recordId, Fixtures.claim(), Fixtures.T0 + 60_000)
        registry.putKey(Fixtures.key())
        media.references[Fixtures.recordId] = image
        val page = assertIs<ProofPageState.Found>(vm(Fixtures.recordId.grouped()).state.value).page
        assertEquals(Verdict.ORIGINAL, page.verdict)
        assertNull(page.qualifier)
        assertNull(page.badge)
        assertEquals(image, page.image)
        assertEquals(0.75f, page.aspect)
        assertEquals(listOf("Captured", "Location", "Device", "Anchored"), page.facts.map { it.label })
        assertEquals("Nov 18, 2026 · 2:12 PM · GNSS time", page.facts[0].value)
        assertEquals("near 39.7° N, 105.0° W · precise sealed", page.facts[1].value)
        assertEquals("Android phone · StrongBox · attested", page.facts[2].value)
        assertEquals("2:13 PM · Internet Computer", page.facts[3].value)
        assertNull(page.warning)
    }

    @Test
    fun recordWithoutReferenceWasRemoved() {
        registry.putRecord(Fixtures.recordId, Fixtures.claim(), 99)
        assertEquals(ProofPageState.Removed(99), vm().state.value)
    }

    @Test
    fun offlineRegistryIsReportedAndRetryRecovers() = runTest {
        registry.offline = true
        val vm = vm()
        assertEquals(ProofPageState.Offline, vm.state.value)
        registry.offline = false
        registry.putRecord(Fixtures.recordId, Fixtures.claim())
        media.references[Fixtures.recordId] = image
        vm.retry()
        assertIs<ProofPageState.Found>(vm.state.value)
    }

    @Test
    fun blurredDerivativeMatchesWithDeclaredBlurs() {
        val rec = AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(kind = blurred), Fixtures.T0)
        val page = ProofPageViewModel.page(rec, null, image, TimeText.UTC)
        assertEquals(Verdict.MATCHES_WITH_CHANGES, page.verdict)
        assertEquals("Matches an original", page.headline)
        assertEquals("with 2 declared blurs", page.qualifier)
        assertEquals("the photographer's own edits", page.qualifierNote)
        assertEquals("2 declared blurs", page.badge)
        assertEquals("Captured on a genuine phone. Only the 2 declared blurs differ.", page.summary)
        assertFalse(page.dev)
    }

    @Test
    fun devRecordsNeverClaimAGenuinePhone() {
        val devKey = Fixtures.key(SecurityLevel.DEV, Provisioning.DEV)
        val blurredPage = ProofPageViewModel.page(AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(kind = blurred), Fixtures.T0), devKey, image, TimeText.UTC)
        assertTrue(blurredPage.dev)
        assertEquals("Captured on a development device. Only the 2 declared blurs differ.", blurredPage.summary)
        val original = ProofPageViewModel.page(AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(), Fixtures.T0), devKey, image, TimeText.UTC)
        assertEquals("Captured on a development device. Unaltered since capture.", original.summary)
        assertTrue(Proves.Dev.none { it.proven && it.text.contains("Genuine") })
        assertTrue(Proves.Dev.first().text.startsWith("Development device"))
    }

    @Test
    fun unknownAreaCodesFallBackToTheRawCode() {
        val rec = AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(coarse = "9xj6a"), Fixtures.T0)
        assertEquals("Area 9xj6a · precise sealed", ProofPageViewModel.facts(rec, null, TimeText.UTC)[1].value)
    }

    @Test
    fun mixedEditsAreCountedTogether() {
        val kind = ClaimKind.Derivative(Fixtures.otherId, Fixtures.captureId, listOf(Fixtures.blur(), Edit.Tone(1, 1, 1)))
        val page = ProofPageViewModel.page(AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(kind = kind), Fixtures.T0), null, image, TimeText.UTC)
        assertEquals("with 2 declared edits", page.qualifier)
    }

    @Test
    fun presentationOnlyDerivativeIsOriginal() {
        val kind = ClaimKind.Derivative(Fixtures.otherId, Fixtures.captureId, listOf(Edit.Resize(100, 100)))
        val page = ProofPageViewModel.page(AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(kind = kind), Fixtures.T0), null, image, TimeText.UTC)
        assertEquals(Verdict.ORIGINAL, page.verdict)
    }

    @Test
    fun warningsSurfaceRevocationLateAnchorAndDev() {
        val revoked = Fixtures.key().copy(history = KeyHistory.of(listOf(StatusChange(KeyStatus.ACTIVE, 0), StatusChange(KeyStatus.REVOKED, Fixtures.T0 * 2)))!!)
        val late = AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(gnssMs = null), Fixtures.T0 + 2 * 86_400_000)
        val w = ProofPageViewModel.page(late, revoked, image, TimeText.UTC).warning!!
        assertTrue(w.contains("revoked"))
        assertTrue(w.contains("more than a day"))
        val dev = ProofPageViewModel.page(AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(), Fixtures.T0), Fixtures.key(SecurityLevel.DEV, Provisioning.DEV), image, TimeText.UTC)
        assertTrue(dev.warning!!.startsWith("DEV"))
        assertTrue(dev.facts[2].value.contains("dev attestation"))
    }

    @Test
    fun factsWithoutGnssOrCoarseArea() {
        val rec = AnchoredRecord(1, Fixtures.recordId, Fixtures.claim(gnssMs = null, coarse = null), Fixtures.T0)
        val facts = ProofPageViewModel.facts(rec, null, TimeText.UTC)
        assertTrue(facts[0].value.endsWith("phone clock"))
        assertEquals("Area hidden · precise sealed", facts[1].value)
    }
}
