package app.witness.core.designsystem

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class WitnessPaletteTest {
    private val light = WitnessPalette.Light
    private val camera = WitnessPalette.Camera

    @Test
    fun lightPaletteUsesPaperTokens() {
        assertEquals(WitnessColors.Paper, light.paper)
        assertEquals(WitnessColors.Surface, light.surface)
        assertEquals(WitnessColors.Ink, light.ink)
        assertEquals(WitnessColors.Ink2, light.ink2)
        assertEquals(WitnessColors.Muted, light.muted)
        assertFalse(light.isDark)
    }

    @Test
    fun cameraPaletteIsDarkWithProvenGreen() {
        assertTrue(camera.isDark)
        assertEquals(WitnessColors.Camera, camera.paper)
        assertEquals(WitnessColors.White, camera.ink)
        assertEquals(WitnessColors.ProvenDark, camera.ok)
        assertEquals(WitnessColors.ProvenDark, camera.okText)
        assertEquals(light.primary, camera.primary)
        assertEquals(light.ai, camera.ai)
    }

    @Test
    fun toneResolvesEveryTone() {
        val expected = mapOf(
            Tone.Ink to light.ink, Tone.Muted to light.muted, Tone.Primary to light.primary, Tone.Ai to light.ai,
            Tone.Ok to light.ok, Tone.Warn to light.warn, Tone.Crit to light.crit, Tone.Edit to light.edit,
        )
        assertEquals(Tone.entries.toSet(), expected.keys)
        expected.forEach { (tone, color) -> assertEquals(color, light.tone(tone), tone.name) }
    }

    @Test
    fun toneTextDarkensOnlyGreenAndAmber() {
        assertEquals(WitnessColors.OkText, light.toneText(Tone.Ok))
        assertEquals(WitnessColors.WarnText, light.toneText(Tone.Warn))
        Tone.entries.filter { it != Tone.Ok && it != Tone.Warn }.forEach { assertEquals(light.tone(it), light.toneText(it), it.name) }
    }

    @Test
    fun onInkContrastsWithInk() {
        assertEquals(WitnessColors.White, light.onInk)
        assertEquals(WitnessColors.Ink, camera.onInk)
        assertNotEquals(light.ink, light.onInk)
        assertNotEquals(camera.ink, camera.onInk)
    }

    @Test
    fun colorSchemeMirrorsPalette() {
        val scheme = light.toColorScheme()
        assertEquals(light.primary, scheme.primary)
        assertEquals(light.paper, scheme.background)
        assertEquals(light.ink, scheme.onSurface)
        assertEquals(light.crit, scheme.error)
        assertEquals(light.line, scheme.outline)
        assertEquals(camera.paper, camera.toColorScheme().background)
    }
}
