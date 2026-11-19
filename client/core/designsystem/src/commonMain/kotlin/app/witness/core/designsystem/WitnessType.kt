package app.witness.core.designsystem

import androidx.compose.runtime.Immutable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

@Immutable
class WitnessType(val sans: FontFamily, val mono: FontFamily) {
    val hero = sans(34.0, FontWeight.Bold, -0.015)
    val display = sans(28.0, FontWeight.Bold, -0.01)
    val verdict = sans(26.0, FontWeight.Bold, -0.01)
    val pageTitle = sans(24.0, FontWeight.Bold, -0.01)
    val headline = sans(22.0, FontWeight.Bold, -0.01)
    val verdictCompact = sans(21.0, FontWeight.Bold, -0.005)
    val qualifierLarge = sans(18.0, FontWeight.SemiBold)
    val title = sans(20.0, FontWeight.SemiBold)
    val cardHeadline = sans(16.5, FontWeight.SemiBold)
    val section = sans(16.0, FontWeight.SemiBold)
    val cardTitle = sans(15.5, FontWeight.SemiBold)
    val actionTitle = sans(15.0, FontWeight.SemiBold)
    val rowTitle = sans(14.5, FontWeight.SemiBold)
    val bodyLarge = sans(15.0, FontWeight.Normal, lineHeight = 1.4)
    val body = sans(14.0, FontWeight.Normal, lineHeight = 1.4)
    val bodyStrong = sans(14.0, FontWeight.SemiBold)
    val bodySmall = sans(13.5, FontWeight.Normal, lineHeight = 1.4)
    val bodySmallStrong = sans(13.5, FontWeight.SemiBold)
    val bodyCompact = sans(13.0, FontWeight.Normal, lineHeight = 1.35)
    val subtitle = sans(12.5, FontWeight.Normal, lineHeight = 1.4)
    val labelStrong = sans(12.5, FontWeight.SemiBold)
    val tileTitle = sans(13.0, FontWeight.SemiBold)
    val label = sans(12.5, FontWeight.Medium)
    val caption = sans(12.0, FontWeight.Normal, lineHeight = 1.35)
    val captionStrong = sans(12.0, FontWeight.SemiBold)
    val meta = sans(11.5, FontWeight.Normal)
    val overline = sans(11.0, FontWeight.SemiBold, 0.08)
    val button = sans(14.5, FontWeight.SemiBold)
    val chip = sans(12.0, FontWeight.SemiBold)
    val nav = sans(12.0, FontWeight.Medium)
    val wordmark = sans(22.0, FontWeight.Bold, -0.015)
    val proof = TextStyle(fontFamily = mono, fontSize = 12.sp, fontWeight = FontWeight.Normal)
    val proofSmall = TextStyle(fontFamily = mono, fontSize = 11.sp, fontWeight = FontWeight.Normal)
    val proofStrong = TextStyle(fontFamily = mono, fontSize = 12.5.sp, fontWeight = FontWeight.Medium)

    private fun sans(size: Double, weight: FontWeight, tracking: Double = 0.0, lineHeight: Double = 1.3) = TextStyle(
        fontFamily = sans,
        fontSize = size.sp,
        fontWeight = weight,
        letterSpacing = tracking.em,
        lineHeight = lineHeight.em,
    )

    companion object {
        val Fallback = WitnessType(FontFamily.SansSerif, FontFamily.Monospace)
    }
}
