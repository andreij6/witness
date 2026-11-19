package app.witness.feature.cases

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import kotlin.test.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w412dp-h915dp-xxhdpi")
class CaseScreensRenderTest {
    private val tints = listOf(Color(0xFF6B7A8F), Color(0xFF8A6F5A), Color(0xFF9AA3A8), Color(0xFF5E7D63))
    private val image: RecordImage = { id -> Box(Modifier.fillMaxSize().background(tints[(id.toBytes()[0].toInt() and 3)])) }
    private fun rid(n: Int) = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (n + it).toByte() })
    private val noop = {}
    private val id = LocalCaseId(1)

    private val cases = CasesUiState(
        CaseFilter.ALL,
        listOf(
            CaseCardUi(LocalCaseId(1), "Fender bender", "Today · Oak St & 5th Ave · 4 items", 3, 1, listOf(rid(0), rid(1), rid(2)), null),
            CaseCardUi(LocalCaseId(2), "Kitchen ceiling leak", "Nov 12 · 6 items", 6, 0, listOf(rid(3)), null),
            CaseCardUi(LocalCaseId(3), "Move-out walkthrough", "Nov 2 · 12 items · Expires Nov 25", 12, 0, listOf(rid(2)), "Shared · 1 link"),
        ),
        loading = false,
    )

    private fun report(sheet: CaseSheet? = null) = CaseReportUiState(
        caseId = id,
        loading = false,
        title = "Fender bender",
        subtitle = "Oak St & 5th Ave · Today 2:31 PM",
        templateId = TemplateId.CAR_ACCIDENT,
        reportLabel = "Report built on-device · Car accident",
        evidenceTitle = "Evidence · 4 sealed",
        anchoringLine = "3 anchored · 1 queued",
        items = listOf(0, 1, 2, 3).map { EvidenceItemUi(rid(it), if (it == 3) WitnessState.Queued else WitnessState.Anchored) },
        summary = "At 2:31 PM a silver sedan struck the rear of my stopped hatchback at the Oak St stop sign. No injuries.",
        sheet = sheet,
    )

    private val actions = CaseReportActions(
        {}, {}, {}, noop, noop, noop, {}, {}, {}, {}, noop, noop, noop, {}, noop, noop, noop, noop, noop,
    )

    private fun check(name: String, content: @androidx.compose.runtime.Composable () -> Unit) {
        val bmp = Snapshot.render(name, content)
        assertTrue(Snapshot.distinctColors(bmp) > 8, "$name rendered blank")
    }

    @Test
    fun c4Cases() = check("c4-cases") { CasesScreen(cases, {}, {}, noop, noop, noop, image = image) }

    @Test
    fun c4Empty() = check("c4-cases-empty") { CasesScreen(CasesUiState(loading = false), {}, {}, noop, noop, noop) }

    @Test
    fun c5Case() = check("c5-case") { CaseReportScreen(report(), actions, image = image) }

    @Test
    fun c5CaseAiDraftedSummary() = check("c5-case-ai-summary") {
        CaseReportScreen(report().copy(summarySource = SummarySource.GENERATED), actions, image = image)
    }

    @Test
    fun c5CaseWrittenSummary() = check("c5-case-written-summary") {
        CaseReportScreen(report().copy(summarySource = SummarySource.WRITTEN), actions, image = image)
    }

    @Test
    fun c6ReportType() = check("c6-report-template") {
        CaseReportScreen(report(CaseSheet.ReportTypes(ReportType.entries.toList(), TemplateId.CAR_ACCIDENT, TemplateId.CAR_ACCIDENT)), actions, image = image)
    }

    @Test
    fun c7Flag() = check("c7-flag-ai-output") {
        CaseReportScreen(report(CaseSheet.Flag(GeneratedOutput("impact", "Impact point", "Rear, driver side"))), actions, image = image)
    }

    @Test
    fun c2Switcher() = check("c2-case-switcher") {
        Box(Modifier.fillMaxSize().background(Color(0xFF3A4048))) {
            WitnessSheet(true, noop) {
                CaseSwitcherContent(
                    CaseSwitcherUiState(
                        listOf(
                            CaseOptionUi(LocalCaseId(1), "Fender bender", "Car accident · 4 items", true),
                            CaseOptionUi(LocalCaseId(2), "Kitchen ceiling leak", "Property damage · 6 items", false),
                            CaseOptionUi(LocalCaseId(3), "Move-out walkthrough", "Rental condition · 12 items", false),
                        ),
                        loading = false,
                    ),
                    {}, noop,
                )
            }
        }
    }

    @Test
    fun c3NewCase() = check("c3-new-case") {
        Box(Modifier.fillMaxSize().background(Color(0xFFF4F3EF))) {
            WitnessSheet(true, noop) { NewCaseContent(NewCaseUiState("Fender bender — Oak & 5th", ReportType.CAR_ACCIDENT), {}, {}, noop) }
        }
    }

    @Test
    fun c1CameraOverlay() = check("c1-camera-case") {
        Box(Modifier.fillMaxSize().background(Color(0xFF59636E))) {
            CaseCameraOverlay(
                CaseCameraUiState(true, id, "Case: Fender bender", EvidenceCoach.coach(TemplateId.CAR_ACCIDENT, 2, 0)),
                noop, noop, noop, noop,
                coachPadding = androidx.compose.foundation.layout.PaddingValues(start = 16.dp, end = 16.dp, bottom = 217.dp),
            )
        }
    }
}
