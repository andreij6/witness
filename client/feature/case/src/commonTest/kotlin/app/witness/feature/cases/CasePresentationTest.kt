package app.witness.feature.cases

import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Case
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.CaseSummary
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.LocalCaseId
import app.witness.core.model.TemplateId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CasePresentationTest {
    private val id = LocalCaseId(1)
    private val day = CaseDates.DAY_MS

    private fun share(expiresAt: Long?) = CaseShare(CaseId.of(0, ByteArray(CaseId.RANDOM_SIZE)), "https://x", NOW, expiresAt, 1)

    private fun case(shares: List<CaseShare> = emptyList(), createdAt: Long = NOW - HOUR_MS) =
        Case(id, "Fender bender", TemplateId.CAR_ACCIDENT, createdAt, shares = shares)

    @Test
    fun anchoredMapsToAnchored() = assertEquals(WitnessState.Anchored, CasePresentation.evidenceState(capture(1, id)))

    @Test
    fun sealedAndQueuedMapToQueued() {
        assertEquals(WitnessState.Queued, CasePresentation.evidenceState(capture(1, id, AnchoringStatus.SEALED)))
        assertEquals(WitnessState.Queued, CasePresentation.evidenceState(capture(1, id, AnchoringStatus.QUEUED)))
        assertEquals(WitnessState.Queued, CasePresentation.evidenceState(capture(1, id, AnchoringStatus.FAILED)))
    }

    @Test
    fun importsAreUnverifiedEvenIfAnchored() =
        assertEquals(WitnessState.Unverified, CasePresentation.evidenceState(capture(1, id, origin = CaptureOrigin.IMPORT)))

    @Test
    fun unverifiedStatusMapsToUnverified() =
        assertEquals(WitnessState.Unverified, CasePresentation.evidenceState(capture(1, id, AnchoringStatus.UNVERIFIED)))

    @Test
    fun activeSharesExcludeExpired() =
        assertEquals(2, CasePresentation.activeShareCount(case(listOf(share(NOW + 1), share(null), share(NOW))), NOW))

    @Test
    fun soonestExpiryIgnoresNeverAndExpired() =
        assertEquals(NOW + 5, CasePresentation.soonestExpiry(case(listOf(share(null), share(NOW + 9), share(NOW + 5), share(NOW - 1))), NOW))

    @Test
    fun soonestExpiryNullWhenAllNever() = assertNull(CasePresentation.soonestExpiry(case(listOf(share(null))), NOW))

    @Test
    fun sharedLabelPluralizes() {
        assertNull(CasePresentation.sharedLabel(0))
        assertEquals("Shared · 1 link", CasePresentation.sharedLabel(1))
        assertEquals("Shared · 3 links", CasePresentation.sharedLabel(3))
    }

    @Test
    fun placesPickNewestNonBlankPerCase() {
        val other = LocalCaseId(2)
        val places = CasePresentation.places(
            listOf(
                capture(1, id, createdAtMs = 10, place = "Old"),
                capture(2, id, createdAtMs = 30, place = " "),
                capture(3, id, createdAtMs = 20, place = "Oak St & 5th Ave"),
                capture(4, other, createdAtMs = 5, place = "Kitchen"),
                capture(5, null, createdAtMs = 50, place = "Loose"),
            ),
        )
        assertEquals(mapOf(id to "Oak St & 5th Ave", other to "Kitchen"), places)
    }

    @Test
    fun cardMetaMatchesDesign() {
        val card = CasePresentation.card(CaseSummary(case(), 4, 3, 1, listOf(record(1))), "Oak St & 5th Ave", NOW, UtcOffset.UTC)
        assertEquals("Today · Oak St & 5th Ave · 4 items", card.meta)
        assertEquals(3, card.anchored)
        assertEquals(1, card.queued)
        assertNull(card.sharedLabel)
    }

    @Test
    fun sharedCardShowsExpiry() {
        val c = case(listOf(share(NOW + 6 * day)), createdAt = NOW - 17 * day)
        val card = CasePresentation.card(CaseSummary(c, 12, 12, 0, emptyList()), null, NOW, UtcOffset.UTC)
        assertEquals("Nov 2 · 12 items · Expires Nov 25", card.meta)
        assertEquals("Shared · 1 link", card.sharedLabel)
        assertTrue(card.shared)
    }

    @Test
    fun singleItemIsSingular() =
        assertEquals("Today · 1 item", CasePresentation.card(CaseSummary(case(), 1, 1, 0, emptyList()), null, NOW, UtcOffset.UTC).meta)

    @Test
    fun filterSplitsOpenAndShared() {
        val open = CaseCardUi(LocalCaseId(1), "a", "", 0, 0, emptyList(), null)
        val shared = open.copy(id = LocalCaseId(2), sharedLabel = "Shared · 1 link")
        val all = listOf(open, shared)
        assertEquals(all, CasePresentation.filter(all, CaseFilter.ALL))
        assertEquals(listOf(open), CasePresentation.filter(all, CaseFilter.OPEN))
        assertEquals(listOf(shared), CasePresentation.filter(all, CaseFilter.SHARED))
        assertEquals(emptyList(), CasePresentation.filter(emptyList(), CaseFilter.SHARED))
    }

    @Test
    fun subtitleJoinsPlaceAndTime() {
        val c = case(createdAt = NOW - 60_000)
        assertEquals("Oak St & 5th Ave · Today 2:31 PM", CasePresentation.caseSubtitle(c, "Oak St & 5th Ave", NOW, UtcOffset.UTC))
        assertEquals("Today 2:31 PM", CasePresentation.caseSubtitle(c, null, NOW, UtcOffset.UTC))
    }

    @Test
    fun optionSubtitleUsesTemplateLabel() = assertEquals("Car accident · 4 items", CasePresentation.optionSubtitle(case(), 4))
}
