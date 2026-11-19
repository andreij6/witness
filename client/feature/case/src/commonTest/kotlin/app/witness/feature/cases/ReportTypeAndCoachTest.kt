package app.witness.feature.cases

import app.witness.core.domain.fake.FakeTemplateCatalog
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReportTypeTest {
    @Test
    fun ofResolvesBundledIds() {
        assertEquals(ReportType.CAR_ACCIDENT, ReportType.of(TemplateId.CAR_ACCIDENT))
        assertEquals(ReportType.GENERAL, ReportType.of(TemplateId.GENERAL))
    }

    @Test
    fun ofUnknownIsNull() = assertNull(ReportType.of(TemplateId("boat")))

    @Test
    fun labelPrefersDesignCopy() = assertEquals("Other", ReportType.label(TemplateId.GENERAL, FakeTemplateCatalog.BUNDLED))

    @Test
    fun labelFallsBackToSchemaTitle() =
        assertEquals("Boat damage", ReportType.label(TemplateId("boat"), listOf(TemplateSchema(TemplateId("boat"), "Boat damage", emptyList(), emptyList()))))

    @Test
    fun labelHumanizesUnknownIds() = assertEquals("Water heater leak", ReportType.label(TemplateId("water_heater-leak")))

    @Test
    fun humanizeKeepsDegenerateInput() {
        assertEquals("", ReportType.humanize(""))
        assertEquals("__", ReportType.humanize("__"))
    }

    @Test
    fun optionsFollowDesignOrderWithTeaser() =
        assertEquals(ReportType.entries.toList(), ReportType.options(FakeTemplateCatalog.BUNDLED))

    @Test
    fun optionsHideTemplatesMissingFromCatalog() {
        val only = FakeTemplateCatalog.BUNDLED.filter { it.id == TemplateId.CAR_ACCIDENT }
        assertEquals(listOf(ReportType.CAR_ACCIDENT, ReportType.WORKPLACE_INCIDENT), ReportType.options(only))
    }

    @Test
    fun optionsWithEmptyCatalogStillShowTeaser() = assertEquals(listOf(ReportType.WORKPLACE_INCIDENT), ReportType.options(emptyList()))

    @Test
    fun onlyWorkplaceIsUnavailable() = assertEquals(listOf(ReportType.WORKPLACE_INCIDENT), ReportType.entries.filter { !it.available })

    @Test
    fun situationsMatchNewCaseChips() =
        assertEquals(listOf("Car accident", "Property damage", "Rental home", "Other"), ReportType.situations.map { it.chipLabel })

    @Test
    fun templateIdsAreUnique() = assertEquals(ReportType.entries.size, ReportType.entries.map { it.templateId }.toSet().size)
}

class EvidenceCoachTest {
    @Test
    fun carAccidentHasFiveSteps() = assertEquals(5, EvidenceCoach.steps(TemplateId.CAR_ACCIDENT).size)

    @Test
    fun unknownTemplateFallsBackToGeneral() = assertEquals(EvidenceCoach.steps(TemplateId.GENERAL), EvidenceCoach.steps(TemplateId("boat")))

    @Test
    fun everyBundledTemplateHasSteps() =
        FakeTemplateCatalog.BUNDLED.forEach { assertTrue(EvidenceCoach.steps(it.id).isNotEmpty(), it.id.value) }

    @Test
    fun designStateIsThirdOfFive() {
        val coach = EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, captured = 2, skipped = 0)!!
        assertEquals("Now get a wide shot showing both cars and the intersection.", coach.prompt)
        assertEquals("3 of 5", coach.progressLabel)
        assertEquals(3, coach.filled)
    }

    @Test
    fun skipsAdvanceLikeCaptures() =
        assertEquals(EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 2, 0), EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 1, 1))

    @Test
    fun firstStepWhenNothingCaptured() = assertEquals(1, EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 0, 0)!!.step)

    @Test
    fun doneWhenAllStepsCovered() = assertNull(EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 3, 2))

    @Test
    fun currentIgnoresNegativeCounts() = assertEquals(0, EvidenceCoach.current(-5, -5, 3))

    @Test
    fun currentDoesNotOverflow() = assertNull(EvidenceCoach.current(Int.MAX_VALUE, Int.MAX_VALUE, 3))

    @Test
    fun currentWithNoStepsIsNull() = assertNull(EvidenceCoach.current(0, 0, 0))

    @Test
    fun lastStepIsReachable() {
        val coach = EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 4, 0)!!
        assertEquals(5, coach.step)
        assertFalse(coach.prompt.isBlank())
    }
}
