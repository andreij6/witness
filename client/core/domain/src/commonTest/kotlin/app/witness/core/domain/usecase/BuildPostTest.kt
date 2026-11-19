package app.witness.core.domain.usecase

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.ImageFormat
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class BuildPostTest {
    private val opening = LocationOpening(ByteString.of(ByteArray(32)), 397_392_000, -1_049_903_000, 500)

    private suspend fun setup(): Pair<TestGraph, Capture> {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.captures.update(c.copy(place = "Denver area"))
        g.captures.putOpening(c.recordId, opening)
        return g to g.captures.get(c.recordId)!!
    }

    @Test
    fun draftUsesRulesWhenAssistantUnavailable() = runTest {
        val (g, c) = setup()
        g.detector.proposals = mapOf(
            c.recordId to listOf(
                BlurProposal(Rect(100, 500, 50, 50), BlurClass.PLATE),
                BlurProposal(Rect(10, 20, 50, 50), BlurClass.FACE),
                BlurProposal(Rect(5000, 5000, 10, 10), BlurClass.OTHER),
                BlurProposal(Rect(4000, 3000, 100, 100), BlurClass.HOUSE_NO),
            ),
        )
        val d = g.post.draft(c.recordId)
        assertEquals(listOf(BlurClass.FACE, BlurClass.PLATE, BlurClass.HOUSE_NO), d.blurs.map { it.blurClass })
        assertEquals(Rect(4000, 3000, 32, 24), d.blurs.last().rect)
        assertEquals("Photo taken in Denver area. 1 face, 1 license plate and 1 house number are blurred.", d.altText)
        assertEquals("", d.caption)
        assertFalse(d.aiGenerated)
        assertEquals(LocationPrecision.AREA, d.precision)
        assertEquals("9xj64", d.coarse)
        assertFalse(d.locationWarning)
        assertTrue(g.assistant.contexts.isEmpty())
    }

    @Test
    fun draftUsesAssistantTextWhenAvailable() = runTest {
        val (g, c) = setup()
        g.assistant.available = true
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.GENERAL, emptyList(), caption = "  Hail on Main St  ", altText = "Hail stones on a car hood") }
        val d = g.post.draft(c.recordId)
        assertEquals("Hail stones on a car hood", d.altText)
        assertEquals("Hail on Main St", d.caption)
        assertTrue(d.aiGenerated)
        assertEquals(listOf(c.recordId), g.assistant.contexts.single().records)
    }

    @Test
    fun blankOrFailingAssistantFallsBackToRules() = runTest {
        val (g, c) = setup()
        g.assistant.available = true
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.GENERAL, emptyList(), altText = "   ") }
        assertFalse(g.post.draft(c.recordId).aiGenerated)
        g.assistant.failure = IllegalStateException("model busy")
        val d = g.post.draft(c.recordId)
        assertFalse(d.aiGenerated)
        assertTrue(d.altText.startsWith("Photo"))
        assertEquals(1, g.telemetry.errors.size)
    }

    @Test
    fun assistantDisabledInSettingsIsNeverCalled() = runTest {
        val (g, c) = setup()
        g.assistant.available = true
        g.settings.update { it.copy(aiAssistant = false) }
        g.post.draft(c.recordId)
        assertTrue(g.assistant.contexts.isEmpty())
    }

    @Test
    fun detectorFailureMeansNoBlurs() = runTest {
        val (g, c) = setup()
        g.detector.failure = IllegalStateException("ml kit missing")
        assertTrue(g.post.draft(c.recordId).blurs.isEmpty())
    }

    @Test
    fun precisionControlsCoarseLocationAndWarning() = runTest {
        val (g, c) = setup()
        val d = g.post.draft(c.recordId)
        assertNull(g.post.withPrecision(d, LocationPrecision.HIDDEN).coarse)
        val n = g.post.withPrecision(d, LocationPrecision.NEIGHBORHOOD)
        assertEquals("9xj64f", n.coarse)
        assertTrue(n.locationWarning)
        val e = g.post.withPrecision(d, LocationPrecision.EXACT)
        assertEquals(9, e.coarse!!.length)
        assertTrue(e.locationWarning)
    }

    @Test
    fun noOpeningMeansNoCoarse() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        assertNull(g.post.draft(c.recordId).coarse)
    }

    @Test
    fun toggleBlurFlipsOneAndIgnoresBadIndex() = runTest {
        val (g, c) = setup()
        g.detector.proposals = mapOf(c.recordId to listOf(BlurProposal(Rect(0, 0, 10, 10), BlurClass.FACE), BlurProposal(Rect(20, 0, 10, 10), BlurClass.FACE)))
        val d = g.post.draft(c.recordId)
        val t = g.post.toggleBlur(d, 1)
        assertEquals(listOf(true, false), t.blurs.map { it.enabled })
        assertEquals(listOf(true, true), g.post.toggleBlur(t, 1).blurs.map { it.enabled })
        assertSame(d, g.post.toggleBlur(d, 5))
        assertSame(d, g.post.toggleBlur(d, -1))
    }

    @Test
    fun derivativeEditsOrderBlursThenGeometryThenTone() = runTest {
        val (g, c) = setup()
        g.detector.proposals = mapOf(c.recordId to listOf(BlurProposal(Rect(0, 0, 24, 24), BlurClass.FACE), BlurProposal(Rect(0, 100, 10, 10), BlurClass.PLATE, enabled = false)))
        val d = g.post.draft(c.recordId).copy(
            adjustments = listOf(
                Edit.Recompress(ImageFormat.JPEG, 85),
                Edit.Tone(10, 0, 0),
                Edit.Crop(Rect(0, 0, 3024, 3024)),
                Edit.Blur(Rect(0, 0, 1, 1), BlurClass.OTHER, 1),
                Edit.Rotate(150),
            ),
        )
        val edits = BuildPost.derivativeEdits(d)
        assertEquals(listOf(EditKind.BLUR, EditKind.CROP, EditKind.ROTATE, EditKind.TONE, EditKind.RECOMPRESS), edits.map { it.kind })
        assertEquals(BuildPost.MIN_SIGMA_E2, (edits[0] as Edit.Blur).sigmaE2)
    }

    @Test
    fun derivativeEditsValidate() {
        val base = app.witness.core.model.PostDraft(RecordId.fromBytes(ByteArray(10)), Dims(100, 100), emptyList(), LocationPrecision.HIDDEN, null, "a", "")
        assertFailsWith<PostException> { BuildPost.derivativeEdits(base.copy(adjustments = listOf(Edit.Rotate(5000)))) }
        assertFailsWith<PostException> { BuildPost.derivativeEdits(base.copy(adjustments = List(65) { Edit.Tone(it, 0, 0) })) }
        assertEquals(emptyList(), BuildPost.derivativeEdits(base))
    }

    @Test
    fun sigmaScalesWithRegion() {
        assertEquals(BuildPost.MIN_SIGMA_E2, BuildPost.sigmaFor(8, 8))
        assertEquals(1_250, BuildPost.sigmaFor(100, 300))
        assertEquals(BuildPost.MAX_SIGMA_E2, BuildPost.sigmaFor(4000, 4000))
    }

    @Test
    fun publishSealsDerivativeWithParentKey() = runTest {
        val (g, c) = setup()
        g.detector.proposals = mapOf(c.recordId to listOf(BlurProposal(Rect(0, 0, 100, 100), BlurClass.FACE)))
        val d = g.post.draft(c.recordId).copy(caption = "  storm  ", adjustments = listOf(Edit.Crop(Rect(0, 0, 3000, 3000))))
        val post = g.post.publish(d)
        assertEquals("https://witness.app/p/${post.recordId.text}", post.proofUrl)
        assertEquals("storm", post.caption)
        assertEquals(c.recordId, post.parent)
        val stored = g.captures.get(post.recordId)!!
        assertEquals(CaptureOrigin.DERIVATIVE, stored.origin)
        assertEquals(AnchoringStatus.QUEUED, stored.status)
        assertEquals(c.keyRef!!.keyId, stored.keyRef!!.keyId)
        assertEquals(1, stored.keyRef!!.counter)
        assertEquals(Dims(3000, 3000), stored.dims)
        assertEquals(c.captureId, stored.captureId)
        val claim = g.engine.claim(post.recordId)!!
        val kind = assertIs<ClaimKind.Derivative>(claim.kind)
        assertEquals(c.recordId, kind.parent)
        assertEquals("9xj64", claim.coarse)
        assertFalse(g.outbox[post.recordId]!!.submission.isOriginal)
        assertEquals(listOf(post.recordId), g.captures.derivativesOf(c.recordId).map { it.recordId })
        assertEquals(1L, g.telemetry.events.last { it.first == AnalyticsEvent.POST_PUBLISHED }.second["blurs"])
    }

    @Test
    fun postingAPostStartsAgainFromTheSealedOriginal() = runTest {
        val (g, c) = setup()
        val first = g.post.publish(g.post.draft(c.recordId).copy(adjustments = listOf(Edit.Crop(Rect(0, 0, 3000, 3000)))))
        val again = g.post.draft(first.recordId)
        assertEquals(c.recordId, again.parent)
        assertEquals("9xj64", again.coarse)
        val second = g.post.publish(again)
        val kind = assertIs<ClaimKind.Derivative>(g.engine.claim(second.recordId)!!.kind)
        assertEquals(c.recordId, kind.parent)
        assertEquals(c.recordId, second.parent)
    }

    @Test
    fun aPostWhoseOriginalIsGoneIsPostedFromItself() = runTest {
        val (g, c) = setup()
        val first = g.post.publish(g.post.draft(c.recordId))
        g.captures.delete(c.recordId)
        assertEquals(first.recordId, g.post.draft(first.recordId).parent)
    }

    @Test
    fun hiddenPrecisionDropsPlace() = runTest {
        val (g, c) = setup()
        val d = g.post.withPrecision(g.post.draft(c.recordId), LocationPrecision.HIDDEN)
        val post = g.post.publish(d)
        assertNull(g.captures.get(post.recordId)!!.place)
        assertNull(post.coarse)
    }

    @Test
    fun publishRequiresParentAndKey() = runTest {
        val (g, c) = setup()
        val d = g.post.draft(c.recordId)
        g.vault.forget(c.keyRef!!.keyId)
        assertFailsWith<PostException> { g.post.publish(d) }
        assertFailsWith<PostException> { g.post.draft(RecordId.fromBytes(ByteArray(10))) }
    }

    @Test
    fun importsCannotBePosted() = runTest {
        val g = TestGraph()
        val id = RecordId.fromBytes(ByteArray(10) { 4 })
        g.captures.insert(Capture(id, null, CaptureOrigin.IMPORT, AnchoringStatus.UNVERIFIED, 0, null, LocationConfidence.LOW, Dims(10, 10)), null, null)
        assertFailsWith<PostException> { g.post.draft(id) }
    }
}
