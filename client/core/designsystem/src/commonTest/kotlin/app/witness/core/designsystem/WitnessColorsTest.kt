package app.witness.core.designsystem

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import kotlin.test.Test
import kotlin.test.assertEquals

class WitnessColorsTest {
    private val designTokens = mapOf(
        "PAPER" to (WitnessColors.Paper to 0xFFF4F3EF),
        "SURF" to (WitnessColors.Surface to 0xFFFFFFFF),
        "SURF2" to (WitnessColors.Surface2 to 0xFFEDEBE5),
        "SURF3" to (WitnessColors.Surface3 to 0xFFE3E0D8),
        "LINE" to (WitnessColors.Line to 0xFFE0DCD2),
        "INK" to (WitnessColors.Ink to 0xFF15171B),
        "TX2" to (WitnessColors.Ink2 to 0xFF575D67),
        "MUTED" to (WitnessColors.Muted to 0xFF666B75),
        "PRIMARY" to (WitnessColors.Primary to 0xFF2D3BE6),
        "ON_PRIMARY" to (WitnessColors.OnPrimary to 0xFFFFFFFF),
        "AI" to (WitnessColors.Ai to 0xFF7A3FF2),
        "OK" to (WitnessColors.Ok to 0xFF0A8754),
        "OK_T" to (WitnessColors.OkText to 0xFF076E44),
        "WARN" to (WitnessColors.Warn to 0xFFB4690E),
        "WARN_T" to (WitnessColors.WarnText to 0xFF925309),
        "CRIT" to (WitnessColors.Crit to 0xFFC93A32),
        "EDIT" to (WitnessColors.Edit to 0xFF0E7490),
        "CAM" to (WitnessColors.Camera to 0xFF0B0C0E),
        "PROVEN_DARK" to (WitnessColors.ProvenDark to 0xFF4ADE80),
        "CAM_SURFACE" to (WitnessColors.CameraSurface to 0xFF1D2026),
        "CAM_TX2" to (WitnessColors.CameraInk2 to 0xFFA3A8B0),
        "CAM_MUTED" to (WitnessColors.CameraMuted to 0xFF6F747C),
        "ICON_CHECK" to (WitnessColors.IconCheck to 0xFF34D399),
    )

    @Test
    fun tokensMatchDesignLibrary() {
        designTokens.forEach { (name, pair) -> assertEquals(pair.second.toInt(), pair.first.toArgb(), name) }
    }

    @Test
    fun tokensMatchLegacyExpectations() {
        assertEquals(Color(0xFFF4F3EF), WitnessColors.Paper)
        assertEquals(Color(0xFF15171B), WitnessColors.Ink)
        assertEquals(Color(0xFF2D3BE6), WitnessColors.Primary)
    }

    @Test
    fun tokensAreOpaque() {
        designTokens.values.forEach { assertEquals(1f, it.first.alpha) }
    }

    @Test
    fun meaningColorsAreDistinct() {
        val meaning = listOf(WitnessColors.Primary, WitnessColors.Ai, WitnessColors.Ok, WitnessColors.Warn, WitnessColors.Crit, WitnessColors.Edit)
        assertEquals(meaning.size, meaning.toSet().size)
    }
}
