package app.witness.core.designsystem

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsOff
import androidx.compose.ui.test.assertIsOn
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class DesignSystemUiTest {
    @Test
    fun buttonInvokesClickOnlyWhenEnabled() = runComposeUiTest {
        var clicks = 0
        var enabled by mutableStateOf(true)
        setContent { WitnessButton("Share case", { clicks++ }, enabled = enabled, icon = WitnessIcon.Share) }
        onNodeWithText("Share case").performClick()
        assertEquals(1, clicks)
        enabled = false
        waitForIdle()
        onNodeWithText("Share case").performClick()
        assertEquals(1, clicks)
    }

    @Test
    fun everyButtonKindRenders() = runComposeUiTest {
        setContent { ButtonKind.entries.forEach { WitnessButton(it.name, {}, kind = it) } }
        ButtonKind.entries.forEach { onNodeWithText(it.name).assertExists() }
    }

    @Test
    fun toggleFlipsOnClick() = runComposeUiTest {
        setContent {
            var on by remember { mutableStateOf(false) }
            WitnessToggle(on, { on = it }, Modifier.testTag("toggle"))
        }
        onNodeWithTag("toggle").assertIsOff().performClick().assertIsOn()
    }

    @Test
    fun filterChipReportsSelection() = runComposeUiTest {
        setContent {
            var selected by remember { mutableIntStateOf(0) }
            Row { listOf("All", "Posts", "Cases").forEachIndexed { i, s -> FilterChip(s, selected == i, { selected = i }) } }
        }
        onNodeWithText("All").assertIsSelected()
        onNodeWithText("Posts").assertIsNotSelected().performClick().assertIsSelected()
        onNodeWithText("All").assertIsNotSelected()
    }

    @Test
    fun segmentedControlSelectsOption() = runComposeUiTest {
        var chosen = -1
        setContent {
            var selected by remember { mutableIntStateOf(0) }
            SegmentedControl(listOf("No", "Yes", "Not sure"), selected, { selected = it; chosen = it })
        }
        onNodeWithText("Not sure").performClick()
        assertEquals(2, chosen)
        onNodeWithText("Not sure").assertIsSelected()
    }

    @Test
    fun glassSegmentedControlSelectsMode() = runComposeUiTest {
        setContent {
            var selected by remember { mutableIntStateOf(0) }
            GlassSegmentedControl(listOf("Post", "Case"), selected, { selected = it })
        }
        onNodeWithText("Case").performClick().assertIsSelected()
    }

    @Test
    fun sheetShowsContentAndDismissesFromScrim() = runComposeUiTest {
        var dismissed = 0
        setContent {
            Box(Modifier.size(400.dp, 800.dp)) {
                WitnessSheet(true, { dismissed++ }) { SheetHeader("Photo 3 of 4", subtitle = "Fender bender") }
            }
        }
        onNodeWithText("Photo 3 of 4").assertExists()
        onNodeWithContentDescription("Dismiss").performClick()
        assertEquals(1, dismissed)
    }

    @Test
    fun hiddenSheetRendersNothing() = runComposeUiTest {
        setContent { WitnessSheet(false, {}) { Text("Hidden") } }
        onNodeWithText("Hidden").assertDoesNotExist()
    }

    @Test
    fun appBarNavigates() = runComposeUiTest {
        var back = 0
        setContent { WitnessAppBar("Check", subtitle = "Shared from another app", navigationIcon = WitnessIcon.Close, navigationLabel = "Close", onNavigate = { back++ }) }
        onNodeWithText("Shared from another app").assertExists()
        onNodeWithContentDescription("Close").performClick()
        assertEquals(1, back)
    }

    @Test
    fun bottomNavSelectsTabsAndCaptures() = runComposeUiTest {
        var captures = 0
        var selected by mutableIntStateOf(0)
        setContent {
            WitnessBottomNav(NavItem("Home", WitnessIcon.Home), NavItem("Check", WitnessIcon.VerifiedUser), selected, { selected = it }, { captures++ })
        }
        onNodeWithText("Check").performClick()
        assertEquals(1, selected)
        onNodeWithContentDescription("Capture").performClick()
        assertEquals(1, captures)
    }

    @Test
    fun textFieldAcceptsInputAndShowsPlaceholder() = runComposeUiTest {
        setContent {
            var text by remember { mutableStateOf("") }
            WitnessTextField(text, { text = it }, Modifier.testTag("field"), placeholder = "Paste a post or proof link", leadingIcon = WitnessIcon.Link)
        }
        onNodeWithText("Paste a post or proof link").assertExists()
        onNodeWithTag("field").performTextInput("witness.app/p/H4IL-7QK")
        onNodeWithText("witness.app/p/H4IL-7QK").assertExists()
    }

    @Test
    fun qrCodeIsDescribed() = runComposeUiTest {
        setContent { QrCode("https://witness.app/p/H4IL-7QK", size = 160.dp) }
        onNodeWithContentDescription("QR code for https://witness.app/p/H4IL-7QK").assertExists()
    }

    @Test
    fun generatedSurfaceIsMarkedAndFlaggable() = runComposeUiTest {
        var flags = 0
        setContent {
            GeneratedSurface(Modifier.testTag("gen")) {
                AiHeader("Still recommended", onFlag = { flags++ })
                GeneratedTag("Read from photo 3")
            }
        }
        onNodeWithTag("gen").assert(SemanticsMatcher.expectValue(SemanticsProperties.StateDescription, GeneratedDescription))
        onNodeWithText("STILL RECOMMENDED").assertExists()
        onNodeWithContentDescription("Flag generated content").performClick()
        assertEquals(1, flags)
    }

    @Test
    fun sealBadgeAndMarksAreDescribed() = runComposeUiTest {
        setContent {
            SealBadge(WitnessState.Queued)
            AppIcon()
        }
        onNodeWithContentDescription("Queued").assertExists()
        onNodeWithContentDescription("Witness").assertExists()
    }

    @Test
    fun dialogCardConfirmsAndCancels() = runComposeUiTest {
        var confirmed = 0
        var cancelled = 0
        setContent { DialogCard("Delete account?", { cancelled++ }, "Delete", { confirmed++ }, text = "Proofs stay.", destructive = true, icon = WitnessIcon.Delete) }
        onNodeWithText("Delete").performClick()
        onNodeWithText("Cancel").performClick()
        assertEquals(1, confirmed)
        assertEquals(1, cancelled)
    }

    @Test
    fun themeProvidesPalette() = runComposeUiTest {
        var captured: WitnessPalette? = null
        setContent { WitnessCameraTheme { captured = Witness.colors } }
        waitForIdle()
        assertEquals(WitnessPalette.Camera, captured)
    }
}
