package app.witness.core.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.Verdict
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class ComponentUiTest {
    @Test
    fun verdictCardShowsHeadlineQualifierAndBody() = runComposeUiTest {
        setContent { VerdictCard(Verdict.MATCHES_WITH_CHANGES) }
        onNodeWithText("Matches an original", substring = true).assertExists()
        onNodeWithText("with changes", substring = true).assertExists()
        onNodeWithText("No local edits found", substring = true).assertExists()
    }

    @Test
    fun everyVerdictCardRendersItsHeadline() = runComposeUiTest {
        setContent { Verdict.entries.forEach { VerdictCard(it, body = null) } }
        Verdict.entries.forEach { onNodeWithText(it.headline(), substring = true).assertExists() }
    }

    @Test
    fun verdictHeadlineShowsNote() = runComposeUiTest {
        setContent { VerdictHeadline(Verdict.MATCHES_WITH_CHANGES, qualifier = "with 2 declared blurs", note = "the photographer's own edits") }
        onNodeWithText("with 2 declared blurs", substring = true).assertExists()
        onNodeWithText("the photographer's own edits", substring = true).assertExists()
    }

    @Test
    fun proofFactsReportClickedIndex() = runComposeUiTest {
        var clicked = -1
        setContent {
            ProofFacts(
                listOf(
                    ProofFact("Captured", "Nov 18, 2026 · 4:12 PM · GNSS time", WitnessIcon.Schedule),
                    ProofFact("Location", "Denver area · precise sealed", WitnessIcon.NearMe),
                    ProofFact("Content", "Differs in 1 region · plate area", status = FactStatus.Failed),
                ),
                onFactClick = { clicked = it },
            )
        }
        onNodeWithText("Location", substring = true).performClick()
        assertEquals(1, clicked)
    }

    @Test
    fun stackedFactsShowBadges() = runComposeUiTest {
        setContent {
            ProofFacts(
                listOf(ProofFact("Anchor", "Not in the registry yet", WitnessIcon.Verified, badge = FactBadge("Pending", app.witness.core.designsystem.Tone.Warn, WitnessIcon.HourglassTop))),
                title = "What we could check",
                layout = FactLayout.Stacked,
            )
        }
        onNodeWithText("WHAT WE COULD CHECK").assertExists()
        onNodeWithText("Pending", substring = true).assertExists()
    }

    @Test
    fun chainOfCustodyShowsStrengths() = runComposeUiTest {
        setContent {
            ChainOfCustody(
                listOf(
                    CustodyStep(WitnessIcon.PhotoCamera, "Captured", "2:31:52 PM · Pixel 9 Pro · Witness 1.0"),
                    CustodyStep(WitnessIcon.Key, "Key attested", "StrongBox · Google root · not revoked", Strength.Strong),
                    CustodyStep(WitnessIcon.SatelliteAlt, "Location", "Raw satellites agree · ±6 m", Strength.Strong),
                    CustodyStep(WitnessIcon.Schedule, "Time", "GNSS-signed · clock within 1 s", Strength.Medium),
                    CustodyStep(WitnessIcon.Verified, "Anchored", "2:36:10 PM · Internet Computer"),
                ),
            )
        }
        onAllNodesWithText("Strong", substring = true).assertCountEquals(2)
        onNodeWithText("Medium", substring = true).assertExists()
    }

    @Test
    fun provesCardListsDefaultLines() = runComposeUiTest {
        setContent { ProvesCard() }
        onNodeWithText("WHAT THIS PROVES").assertExists()
        onNodeWithContentDescription("Doesn't prove the scene wasn't staged").assertExists()
    }

    @Test
    fun checkProgressExposesProgress() = runComposeUiTest {
        val stages = listOf(
            CheckStage("Reading the file", "JPEG · re-encoded by the platform", StageStatus.Done),
            CheckStage("Searching fingerprints", "3 index shards · 2 answered", StageStatus.Running, 0.66f),
            CheckStage("Reading the watermark"),
        )
        setContent { CheckProgress(stages, Modifier.testTag("progress")) }
        onNodeWithText("Searching fingerprints", substring = true).assertExists()
        onNodeWithTag("progress").assert(SemanticsMatcher.keyIsDefined(SemanticsProperties.ProgressBarRangeInfo))
    }

    @Test
    fun caseRowShowsCounts() = runComposeUiTest {
        setContent { CaseRow("Fender bender", "Today · 4 items", stateCounts(3, 1), 3) { Text("t$it") } }
        onNodeWithText("Fender bender").assertExists()
        onNodeWithText("3").assertExists()
        onNodeWithText("1").assertExists()
        onNodeWithText("t2").assertExists()
    }

    @Test
    fun postCardShowsCheckedBadge() = runComposeUiTest {
        var opened = 0
        setContent { PostCard("Golf-ball hail out of nowhere.", "Today 4:12 PM · Denver area", WitnessState.Anchored, checks = 1284, onClick = { opened++ }) {} }
        onNodeWithText("Checked 1,284×").assertExists()
        onNodeWithText("Golf-ball hail out of nowhere.").performClick()
        assertEquals(1, opened)
    }

    @Test
    fun caseItemRowDescribesState() = runComposeUiTest {
        setContent { CaseItemRow("Photo 4 · Wide shot", "2:34:05 PM · face blurred", WitnessState.Edited) {} }
        onNodeWithText("Photo 4 · Wide shot", substring = true).assertExists()
    }

    @Test
    fun referenceCompareShowsCropAndCaptions() = runComposeUiTest {
        setContent {
            Box(Modifier.size(400.dp, 600.dp)) {
                ReferenceCompare(
                    ComparePane("Your screenshot", "Tap either to compare", captionTone = app.witness.core.designsystem.Tone.Primary),
                    ComparePane("Sealed reference", "With 2 declared blurs", sourceAspect = 896f / 1200f, crop = NormRect(0f, 0.2f, 1f, 0.8f)),
                    startContent = {},
                    endContent = {},
                )
            }
        }
        onNodeWithText("YOUR SCREENSHOT").assertExists()
        onNodeWithText("Crop").assertExists()
        onNodeWithText("With 2 declared blurs").assertExists()
    }

    @Test
    fun webFooterLinksAreClickable() = runComposeUiTest {
        var reports = 0
        setContent { WebFooter(listOf(WebLink("Report this page", { reports++ }, WitnessIcon.Report), WebLink("Contact", {}), WebLink("Check a copy", {}))) }
        onNodeWithText("Report this page").performClick()
        assertEquals(1, reports)
        onNodeWithText(CertifiedNote).assertExists()
    }

    @Test
    fun browserSafeLayoutStacksOnPhones() = runComposeUiTest {
        setContent {
            Box(Modifier.size(412.dp, 900.dp)) {
                BrowserSafeLayout(header = { WebHeader() }, secondary = { Text("secondary ${it.mode}") }) { Text("primary ${it.mode}") }
            }
        }
        onNodeWithText("primary Compact").assertExists()
        onNodeWithText("secondary Compact").assertExists()
        onNodeWithText("Verification").assertExists()
    }

    @Test
    fun browserSafeLayoutSplitsOnDesktop() = runComposeUiTest {
        setContent {
            Box(Modifier.size(1280.dp, 820.dp)) {
                BrowserSafeLayout(secondary = { Text("secondary ${it.mode}") }) { Text("primary ${it.mode}") }
            }
        }
        onNodeWithText("primary Wide").assertExists()
        onNodeWithText("secondary Wide").assertExists()
    }

    @Test
    fun locationPillsRenderText() = runComposeUiTest {
        setContent {
            LocationPill("Denver area", time = "Nov 18, 4:12 PM")
            LocationConfidencePill(LocationConfidence.High)
        }
        onNodeWithText("Denver area · Nov 18, 4:12 PM").assertExists()
        onNodeWithText("Location · High").assertExists()
    }
}
