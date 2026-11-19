package app.witness.core.designsystem

import androidx.compose.ui.graphics.Color
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ComponentColorsTest {
    private val light = WitnessPalette.Light

    @Test
    fun buttonKindsMatchDesignLibrary() {
        assertEquals(ButtonColors(WitnessColors.Primary, WitnessColors.OnPrimary, null), ButtonKind.Filled.colors(light))
        assertEquals(ButtonColors(WitnessColors.Ink, WitnessColors.White, null), ButtonKind.Ink.colors(light))
        assertEquals(ButtonColors(WitnessColors.Surface3, WitnessColors.Ink, null), ButtonKind.Tonal.colors(light))
        assertEquals(ButtonColors(Color.Transparent, WitnessColors.Ink, WitnessColors.Line), ButtonKind.Outlined.colors(light))
        assertEquals(ButtonColors(WitnessColors.Crit, WitnessColors.White, null), ButtonKind.Danger.colors(light))
    }

    @Test
    fun glassButtonIsTranslucentBlack() {
        val glass = ButtonKind.Glass.colors(light)
        assertEquals(GlassAlpha, glass.container.alpha, 0.005f)
        assertEquals(0f, glass.container.red)
        assertEquals(WitnessColors.White, glass.content)
        assertNull(glass.border)
    }

    @Test
    fun inkButtonInvertsOnCamera() {
        val ink = ButtonKind.Ink.colors(WitnessPalette.Camera)
        assertEquals(WitnessColors.White, ink.container)
        assertEquals(WitnessColors.Ink, ink.content)
    }

    @Test
    fun everyButtonKindHasReadableContent() {
        ButtonKind.entries.forEach { kind ->
            val c = kind.colors(light)
            if (c.container.alpha == 1f) kotlin.test.assertNotEquals(c.container, c.content, kind.name)
        }
    }

    @Test
    fun tintedChipUsesTwelvePercentFillAndDarkText() {
        val ok = light.tintedChip(Tone.Ok)
        assertEquals(TintAlpha, ok.container.alpha, 0.005f)
        assertEquals(WitnessColors.Ok, ok.content)
        assertEquals(WitnessColors.OkText, ok.text)
        val crit = light.tintedChip(Tone.Crit)
        assertEquals(WitnessColors.Crit, crit.text)
        assertNull(crit.border)
    }

    @Test
    fun solidAndOutlineChipsKeepTextColor() {
        val solid = light.solidChip(WitnessColors.Edit)
        assertEquals(WitnessColors.Edit, solid.container)
        assertEquals(WitnessColors.White, solid.text)
        val outline = light.outlineChip()
        assertEquals(Color.Transparent, outline.container)
        assertEquals(WitnessColors.Line, outline.border)
        assertEquals(WitnessColors.Ink2, outline.text)
    }
}
