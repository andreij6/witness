package app.witness.core.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.designsystem.GlassPill
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessPalette

enum class LocationConfidence(val label: String, val reason: String, val tone: Tone, val icon: WitnessIcon) {
    High("High", "Raw satellite checks passed", Tone.Ok, WitnessIcon.SatelliteAlt),
    Medium("Medium", "Fused fix with no satellite check", Tone.Warn, WitnessIcon.NearMe),
    Low("Low", "Weak or inconsistent signals", Tone.Crit, WitnessIcon.Warning),
}

fun LocationConfidence.pillText(): String = "Location · $label"

fun locationPillText(area: String, time: String? = null): String = Formatters.joinDot(area, time)

@Composable
fun LocationPill(area: String, modifier: Modifier = Modifier, time: String? = null, onClick: (() -> Unit)? = null) =
    GlassPill(locationPillText(area, time), WitnessIcon.LocationOn, modifier, onClick = onClick)

@Composable
fun LocationConfidencePill(confidence: LocationConfidence, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null) =
    GlassPill(confidence.pillText(), confidence.icon, modifier, iconTint = WitnessPalette.Camera.tone(confidence.tone), onClick = onClick)
