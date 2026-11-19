package app.witness.feature.post

import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.Links
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import app.witness.core.model.Rect
import app.witness.core.model.Settings
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ComposerViewModelTest {
    private val main = UnconfinedTestDispatcher()

    @BeforeTest
    fun setUp() = Dispatchers.setMain(main)

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private val house = BlurProposal(Rect(2900, 1200, 80, 140), BlurClass.HOUSE_NO)
    private val plate = BlurProposal(Rect(2200, 1700, 160, 80), BlurClass.PLATE)

    private fun TestScope.started(vm: ComposerViewModel): ComposerViewModel {
        backgroundScope.launch(main) { vm.state.collect {} }
        return vm
    }

    private fun ComposerViewModel.draft() = state.value.draft!!

    @Test
    fun loadsDraftWithBlursAndSubtitle() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(house, plate), place = "Denver area")))
        val s = vm.state.value
        assertFalse(s.loading)
        assertEquals("Sealed 4:12:09 PM · anchoring…", s.subtitle)
        assertEquals(2, s.enabledBlurs)
        assertEquals("Denver area", s.place)
        assertEquals(LocationPrecision.AREA, s.draft?.precision)
        assertNotNull(s.draft?.coarse)
        assertEquals("witness.app/p/…", s.linkText)
        assertTrue(s.proofLink)
        assertTrue(s.canShare)
    }

    @Test
    fun loadsPhotoBytes() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(image = byteArrayOf(-1, -40, 1))))
        assertEquals(3, vm.state.value.photo?.bytes?.size)
    }

    @Test
    fun missingPhotoIsNull() = runTest(main) {
        val g = PostGraph()
        val p = g.parent()
        val noImages = object : CaptureStore by g.captures {
            override suspend fun readImage(recordId: RecordId): ByteString? = null
        }
        assertNull(started(ComposerViewModel(p.recordId, g.post, noImages)).state.value.photo)
    }

    @Test
    fun subtitleFollowsAnchoring() = runTest(main) {
        val g = PostGraph()
        val p = g.parent()
        val vm = started(g.composer(p, offset = 3_600_000))
        g.captures.setStatus(p.recordId, AnchoringStatus.ANCHORED, Anchor(1, g.clock.nowMs))
        assertEquals("Sealed 5:12:09 PM · anchored", vm.state.value.subtitle)
    }

    @Test
    fun unknownParentFailsToLoad() = runTest(main) {
        val g = PostGraph()
        val vm = started(ComposerViewModel(RecordId.fromBytes(ByteArray(10)), g.post, g.captures))
        assertEquals(PostCopy.LOAD_FAILED, vm.state.value.error)
        assertNull(vm.state.value.draft)
        assertFalse(vm.state.value.canShare)
        vm.dismissError()
        assertNull(vm.state.value.error)
    }

    @Test
    fun importsCannotBePosted() = runTest(main) {
        val g = PostGraph()
        val p = g.parent()
        g.captures.update(p.copy(origin = CaptureOrigin.IMPORT))
        val vm = started(g.composer(p))
        assertEquals(PostCopy.LOAD_FAILED, vm.state.value.error)
    }

    @Test
    fun toggleBlurRemovesAndRestores() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(house, plate))))
        val first = vm.draft().blurs[0]
        vm.toggleBlur(0)
        assertEquals(1, vm.state.value.enabledBlurs)
        assertFalse(vm.draft().blurs[0].enabled)
        assertEquals(first.rect, vm.draft().blurs[0].rect)
        vm.toggleBlur(0)
        assertEquals(2, vm.state.value.enabledBlurs)
    }

    @Test
    fun toggleBlurOutOfRangeIsIgnored() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(house))))
        val before = vm.draft()
        vm.toggleBlur(5)
        vm.toggleBlur(-1)
        assertEquals(before, vm.draft())
    }

    @Test
    fun locationSheetFlow() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openLocation()
        assertEquals(ComposerSheet.Location(LocationPrecision.AREA), vm.state.value.sheet)
        vm.selectPrecision(LocationPrecision.EXACT)
        assertEquals(LocationPrecision.AREA, vm.draft().precision)
        vm.confirmLocation()
        assertNull(vm.state.value.sheet)
        assertEquals(LocationPrecision.EXACT, vm.draft().precision)
        assertEquals(9, vm.draft().coarse?.length)
        assertTrue(vm.draft().locationWarning)
    }

    @Test
    fun hiddenPrecisionDropsCoarse() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openLocation()
        vm.selectPrecision(LocationPrecision.HIDDEN)
        vm.confirmLocation()
        assertNull(vm.draft().coarse)
    }

    @Test
    fun dismissingLocationSheetKeepsPrecision() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openLocation()
        vm.selectPrecision(LocationPrecision.NEIGHBORHOOD)
        vm.dismissSheet()
        assertNull(vm.state.value.sheet)
        assertEquals(LocationPrecision.AREA, vm.draft().precision)
    }

    @Test
    fun selectWithoutSheetIgnored() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.selectPrecision(LocationPrecision.EXACT)
        vm.confirmLocation()
        assertNull(vm.state.value.sheet)
        assertEquals(LocationPrecision.AREA, vm.draft().precision)
    }

    @Test
    fun defaultPrecisionComesFromSettings() = runTest(main) {
        val g = PostGraph(Settings(defaultPrecision = LocationPrecision.HIDDEN))
        val vm = started(g.composer(g.parent()))
        assertEquals(LocationPrecision.HIDDEN, vm.draft().precision)
        assertNull(vm.draft().coarse)
    }

    @Test
    fun altTextEditFlow() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openAltText()
        assertIs<ComposerSheet.AltText>(vm.state.value.sheet)
        vm.editAltText("  Hail   on a lawn ")
        vm.confirmAltText()
        assertEquals("Hail on a lawn", vm.draft().altText)
        assertFalse(vm.draft().aiGenerated)
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun blankAltTextFallsBackToRules() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(plate), place = "Denver")))
        vm.openAltText()
        vm.editAltText("   ")
        vm.confirmAltText()
        assertEquals("Photo taken in Denver. 1 license plate is blurred.", vm.draft().altText)
    }

    @Test
    fun altTextIsBounded() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openAltText()
        vm.editAltText("x".repeat(5_000))
        assertEquals(1_000, (vm.state.value.sheet as ComposerSheet.AltText).text.length)
    }

    @Test
    fun editAltTextWithoutSheetIgnored() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.editAltText("x")
        vm.confirmAltText()
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun captionEditClearsSuggestionAndIsBounded() = runTest(main) {
        val g = PostGraph()
        g.assistant.available = true
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.GENERAL, emptyList(), "Golf-ball hail.", "Hailstones on a lawn.") }
        val vm = started(g.composer(g.parent()))
        assertTrue(vm.state.value.captionSuggested)
        assertEquals("Golf-ball hail.", vm.draft().caption)
        assertTrue(vm.draft().aiGenerated)
        vm.setCaption("y".repeat(9_000))
        assertFalse(vm.state.value.captionSuggested)
        assertEquals(2_200, vm.draft().caption.length)
    }

    @Test
    fun flagAiRemovesGeneratedText() = runTest(main) {
        val g = PostGraph()
        g.assistant.available = true
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.GENERAL, emptyList(), "Golf-ball hail.", "Hailstones on a lawn.") }
        val vm = started(g.composer(g.parent(listOf(house), place = "Denver")))
        vm.flagAi()
        assertEquals("", vm.draft().caption)
        assertEquals("Photo taken in Denver. 1 house number is blurred.", vm.draft().altText)
        assertFalse(vm.draft().aiGenerated)
        assertFalse(vm.state.value.captionSuggested)
        assertEquals(listOf("post_ai_flagged"), g.telemetry.breadcrumbs)
    }

    @Test
    fun flagAiKeepsUserCaption() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.setCaption("mine")
        vm.flagAi()
        assertEquals("mine", vm.draft().caption)
    }

    @Test
    fun proofLinkToggle() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.setProofLink(false)
        assertFalse(vm.state.value.proofLink)
    }

    @Test
    fun editorApplyDeclaresEdits() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(house))))
        vm.openEditor()
        assertNotNull(vm.state.value.editor)
        vm.editor { PhotoEditor.withAspect(it, CropAspect.PORTRAIT) }
        vm.editor { PhotoEditor.straighten(it, 250) }
        vm.applyEditor()
        assertNull(vm.state.value.editor)
        assertEquals(listOf(EditKind.CROP, EditKind.ROTATE), vm.draft().adjustments.map { it.kind })
    }

    @Test
    fun editorCloseDiscards() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openEditor()
        vm.editor { PhotoEditor.straighten(it, 500) }
        vm.closeEditor()
        assertNull(vm.state.value.editor)
        assertTrue(vm.draft().adjustments.isEmpty())
    }

    @Test
    fun editorTransformWithoutEditorIgnored() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.editor { PhotoEditor.straighten(it, 500) }
        vm.applyEditor()
        assertNull(vm.state.value.editor)
        assertTrue(vm.draft().adjustments.isEmpty())
    }

    @Test
    fun editorBlurToolAddsDeclaredBlur() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.openEditor()
        vm.editor { PhotoEditor.tapBlur(it.copy(tool = EditTool.BLUR), 100, 100) }
        vm.applyEditor()
        assertEquals(1, vm.state.value.enabledBlurs)
        assertEquals(BlurClass.OTHER, vm.draft().blurs.single().blurClass)
    }

    @Test
    fun copyLinkPublishesAndEmitsUrl() = runTest(main) {
        val g = PostGraph()
        val p = g.parent(listOf(house, plate))
        val vm = started(g.composer(p))
        vm.copyLink()
        val effect = assertIs<ComposerEffect.CopyLink>(vm.effects.first())
        val post = assertNotNull(vm.state.value.published)
        assertEquals(post.proofUrl, effect.url)
        assertEquals(PostCopy.linkDisplay(post.proofUrl), vm.state.value.linkText)
        val derivative = assertNotNull(g.captures.get(post.recordId))
        assertEquals(p.recordId, derivative.parent)
        assertEquals(2, derivative.edits.count { it is Edit.Blur })
    }

    @Test
    fun linkChipShowsTheConfiguredOriginAndTheRealLinkOnceSealed() = runTest(main) {
        val local = Links.at("http://4qggx-l3777-77775-aaaca-cai.localhost:8090")
        val g = PostGraph(links = local)
        val vm = started(g.composer(g.parent(listOf(house))))
        assertEquals("4qggx-l3777-77775-aaaca-cai.localhost:8090/p/…", vm.state.value.linkText)
        vm.copyLink()
        val effect = assertIs<ComposerEffect.CopyLink>(vm.effects.first())
        val post = assertNotNull(vm.state.value.published)
        assertEquals("http://4qggx-l3777-77775-aaaca-cai.localhost:8090/p/${post.recordId.text}", effect.url)
        assertEquals("4qggx-l3777-77775-aaaca-cai.localhost:8090/p/${post.recordId.text}", vm.state.value.linkText)
    }

    @Test
    fun shareIncludesProofLink() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.setCaption("Hail!")
        vm.share()
        val effect = assertIs<ComposerEffect.Share>(vm.effects.first())
        assertEquals("Hail!\n${effect.post.proofUrl}", effect.text)
    }

    @Test
    fun shareWithoutProofLink() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.setCaption("Hail!")
        vm.setProofLink(false)
        vm.share()
        assertEquals("Hail!", assertIs<ComposerEffect.Share>(vm.effects.first()).text)
    }

    @Test
    fun repeatedShareReusesPost() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.copyLink()
        val a = assertIs<ComposerEffect.CopyLink>(vm.effects.first())
        vm.share()
        val b = assertIs<ComposerEffect.Share>(vm.effects.first())
        assertEquals(a.url, b.post.proofUrl)
        assertEquals(2, g.captures.snapshot.size)
    }

    @Test
    fun sealedChangeAfterPublishRepublishes() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent(listOf(house))))
        vm.copyLink()
        val a = assertIs<ComposerEffect.CopyLink>(vm.effects.first())
        vm.toggleBlur(0)
        assertNull(vm.state.value.published)
        assertEquals("witness.app/p/…", vm.state.value.linkText)
        vm.copyLink()
        val b = assertIs<ComposerEffect.CopyLink>(vm.effects.first())
        assertNotEquals(a.url, b.url)
        assertEquals(3, g.captures.snapshot.size)
    }

    @Test
    fun captionChangeAfterPublishKeepsPost() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        vm.copyLink()
        vm.effects.first()
        vm.setCaption("later")
        assertNotNull(vm.state.value.published)
    }

    @Test
    fun publishFailureShowsError() = runTest(main) {
        val g = PostGraph()
        val p = g.parent()
        val vm = started(g.composer(p))
        g.vault.forget(p.keyRef!!.keyId)
        vm.share()
        assertEquals(PostCopy.PUBLISH_FAILED, vm.state.value.error)
        assertNull(vm.state.value.published)
        assertFalse(vm.state.value.busy)
    }

    @Test
    fun publishRequestsAnchoring() = runTest(main) {
        val g = PostGraph()
        val vm = started(g.composer(g.parent()))
        val before = g.scheduler.requests
        vm.copyLink()
        vm.effects.first()
        assertEquals(before + 1, g.scheduler.requests)
    }
}
