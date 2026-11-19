package app.witness.core.designsystem

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

enum class Tone { Ink, Muted, Primary, Ai, Ok, Warn, Crit, Edit }

@Immutable
data class WitnessPalette(
    val paper: Color,
    val surface: Color,
    val surface2: Color,
    val surface3: Color,
    val line: Color,
    val ink: Color,
    val ink2: Color,
    val muted: Color,
    val primary: Color,
    val onPrimary: Color,
    val ai: Color,
    val ok: Color,
    val okText: Color,
    val warn: Color,
    val warnText: Color,
    val crit: Color,
    val edit: Color,
    val camera: Color,
    val scrim: Color,
    val isDark: Boolean,
) {
    val onInk: Color get() = if (isDark) WitnessColors.Ink else WitnessColors.White

    fun tone(tone: Tone): Color = when (tone) {
        Tone.Ink -> ink
        Tone.Muted -> muted
        Tone.Primary -> primary
        Tone.Ai -> ai
        Tone.Ok -> ok
        Tone.Warn -> warn
        Tone.Crit -> crit
        Tone.Edit -> edit
    }

    fun toneText(tone: Tone): Color = when (tone) {
        Tone.Ok -> okText
        Tone.Warn -> warnText
        else -> tone(tone)
    }

    companion object {
        val Light = WitnessPalette(
            paper = WitnessColors.Paper,
            surface = WitnessColors.Surface,
            surface2 = WitnessColors.Surface2,
            surface3 = WitnessColors.Surface3,
            line = WitnessColors.Line,
            ink = WitnessColors.Ink,
            ink2 = WitnessColors.Ink2,
            muted = WitnessColors.Muted,
            primary = WitnessColors.Primary,
            onPrimary = WitnessColors.OnPrimary,
            ai = WitnessColors.Ai,
            ok = WitnessColors.Ok,
            okText = WitnessColors.OkText,
            warn = WitnessColors.Warn,
            warnText = WitnessColors.WarnText,
            crit = WitnessColors.Crit,
            edit = WitnessColors.Edit,
            camera = WitnessColors.Camera,
            scrim = WitnessColors.Black,
            isDark = false,
        )

        val Camera = Light.copy(
            paper = WitnessColors.Camera,
            surface = WitnessColors.CameraSurface,
            surface2 = WitnessColors.CameraSurface,
            surface3 = WitnessColors.CameraLine,
            line = WitnessColors.CameraLine,
            ink = WitnessColors.White,
            ink2 = WitnessColors.CameraInk2,
            muted = WitnessColors.CameraMuted,
            ok = WitnessColors.ProvenDark,
            okText = WitnessColors.ProvenDark,
            isDark = true,
        )
    }
}
