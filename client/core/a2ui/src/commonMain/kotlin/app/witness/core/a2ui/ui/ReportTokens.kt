package app.witness.core.a2ui.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.WitnessColors

internal object ReportTokens {
    val Surface = WitnessColors.Surface
    val Surface2 = Color(0xFFEDEBE5)
    val Surface3 = Color(0xFFE3E0D8)
    val Line = WitnessColors.Line
    val Ink = WitnessColors.Ink
    val Text2 = Color(0xFF575D67)
    val Muted = WitnessColors.Muted
    val Primary = WitnessColors.Primary
    val Ai = WitnessColors.Ai
    val Ok = WitnessColors.Ok
    val Crit = WitnessColors.Crit
    val White = Color.White

    val Gutter = 18.dp
    val SurfaceRadius = 18.dp
    val SectionGap = 16.dp

    val Overline = TextStyle(fontSize = 11.sp, fontWeight = FontWeight.SemiBold, letterSpacing = 0.08.em, color = Muted)
    val Heading = TextStyle(fontSize = 16.sp, fontWeight = FontWeight.SemiBold, color = Ink)
    val Value = TextStyle(fontSize = 15.5.sp, fontWeight = FontWeight.SemiBold, color = Ink)
    val Caption = TextStyle(fontSize = 12.sp, fontWeight = FontWeight.Medium, color = Ai)
    val Meta = TextStyle(fontSize = 12.sp, color = Muted)
    val ItemDone = TextStyle(fontSize = 13.5.sp, fontWeight = FontWeight.Medium, color = Text2)
    val ItemOpen = TextStyle(fontSize = 13.5.sp, fontWeight = FontWeight.SemiBold, color = Ink)
    val Action = TextStyle(fontSize = 13.sp, fontWeight = FontWeight.SemiBold, color = Primary)
    val Body = TextStyle(fontSize = 12.5.sp, color = Text2, lineHeight = 17.5.sp)
    val Plate = TextStyle(fontSize = 15.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, color = Ink)
    val Chip = TextStyle(fontSize = 11.sp, fontWeight = FontWeight.SemiBold, color = Ai)
    val Choice = TextStyle(fontSize = 13.sp, fontWeight = FontWeight.Medium, color = Text2)
}
