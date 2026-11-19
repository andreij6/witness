package app.witness.core.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf

val LocalWitnessPalette = staticCompositionLocalOf { WitnessPalette.Light }
val LocalWitnessType = staticCompositionLocalOf { WitnessType.Fallback }

object Witness {
    val colors: WitnessPalette
        @Composable @ReadOnlyComposable get() = LocalWitnessPalette.current
    val type: WitnessType
        @Composable @ReadOnlyComposable get() = LocalWitnessType.current
}

@Composable
fun WitnessTheme(palette: WitnessPalette = WitnessPalette.Light, content: @Composable () -> Unit) {
    val sans = interFontFamily()
    val mono = jetBrainsMonoFontFamily()
    val type = remember(sans, mono) { WitnessType(sans, mono) }
    val scheme = remember(palette) { palette.toColorScheme() }
    CompositionLocalProvider(LocalWitnessPalette provides palette, LocalWitnessType provides type) {
        MaterialTheme(colorScheme = scheme, content = content)
    }
}

@Composable
fun WitnessCameraTheme(content: @Composable () -> Unit) = WitnessTheme(WitnessPalette.Camera, content)

fun WitnessPalette.toColorScheme(): ColorScheme {
    val base = if (isDark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = primary,
        onPrimary = onPrimary,
        secondary = ink,
        onSecondary = paper,
        tertiary = ai,
        background = paper,
        onBackground = ink,
        surface = surface,
        onSurface = ink,
        surfaceVariant = surface2,
        onSurfaceVariant = ink2,
        surfaceContainer = surface,
        surfaceContainerHigh = surface2,
        surfaceContainerHighest = surface3,
        outline = line,
        outlineVariant = line,
        error = crit,
        onError = onPrimary,
        scrim = scrim,
    )
}
