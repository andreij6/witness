package app.witness.feature.cases

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import app.witness.core.designsystem.GlassPill
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.overlineText

internal object CoachColors {
    val Fill = Color(0xFF101216).copy(alpha = 0.9f)
    val Accent = Color(0xFFB794FF)
    val Flag = Color(0xFF8E939C)
    val Track = Color(0xFF3A3D45)
    val Meta = Color(0xFFA9ADB5)
    val Skip = Color(0xFFD0D3D8)
}

private val CoachShape = RoundedCornerShape(20.dp)

@Composable
fun CaseCameraOverlay(
    state: CaseCameraUiState,
    onSwitchCase: () -> Unit,
    onNewCase: () -> Unit,
    onSkip: () -> Unit,
    onFlag: () -> Unit,
    modifier: Modifier = Modifier,
    pillsPadding: PaddingValues = PaddingValues(start = 16.dp, top = 138.dp),
    coachPadding: PaddingValues = PaddingValues(start = 16.dp, end = 16.dp, bottom = 26.dp),
) {
    if (!state.visible) return
    Box(modifier.fillMaxSize()) {
        CaseCameraPills(state.pillLabel, onSwitchCase, onNewCase, Modifier.padding(pillsPadding))
        val coach = state.coach
        if (coach != null) EvidenceCoachCard(coach, onSkip, onFlag, Modifier.align(Alignment.BottomCenter).padding(coachPadding))
    }
}

@Composable
fun CaseCameraPills(label: String, onSwitchCase: () -> Unit, onNewCase: () -> Unit, modifier: Modifier = Modifier) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        GlassPill(label, WitnessIcon.Folder, trailing = WitnessIcon.ExpandMore, onClick = onSwitchCase)
        Spacer(Modifier.width(6.dp))
        GlassPill("New case", WitnessIcon.Add, onClick = onNewCase)
    }
}

@Composable
fun EvidenceCoachCard(coach: CoachUi, onSkip: () -> Unit, onFlag: () -> Unit, modifier: Modifier = Modifier) {
    val type = Witness.type
    Column(
        modifier
            .fillMaxWidth()
            .clip(CoachShape)
            .background(CoachColors.Fill)
            .border(1.2.dp, WitnessColors.Ai.copy(alpha = 0.6f), CoachShape)
            .drawBehind {
                val inset = 18.dp.toPx()
                val w = 3.dp.toPx()
                drawRoundRect(WitnessColors.Ai, Offset(0f, inset), Size(w, size.height - inset * 2), CornerRadius(w / 2))
            }
            .padding(start = 16.dp, end = 8.dp, top = 8.dp, bottom = 14.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.AutoAwesome, CoachColors.Accent, size = 18.dp)
            Spacer(Modifier.width(6.dp))
            Text(overlineText("Evidence coach"), Modifier.weight(1f), style = type.overline, color = CoachColors.Accent, maxLines = 1)
            WitnessIconButton(WitnessIcon.Flag, "Flag coach suggestion", onFlag, tint = CoachColors.Flag, size = 32.dp, iconSize = 20.dp)
        }
        Text(
            coach.prompt,
            Modifier.padding(end = 8.dp).semantics { contentDescription = "Evidence coach, step ${coach.progressLabel}: ${coach.prompt}" },
            style = type.actionTitle.merge(TextStyle(lineHeight = 1.35.em)),
            color = WitnessColors.White,
        )
        Spacer(Modifier.height(10.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            repeat(coach.total) { i ->
                Box(Modifier.size(16.dp, 4.dp).clip(RoundedCornerShape(2.dp)).background(if (i < coach.filled) CoachColors.Accent else CoachColors.Track))
                Spacer(Modifier.width(6.dp))
            }
            Spacer(Modifier.width(8.dp))
            Text(coach.progressLabel, Modifier.weight(1f), style = type.meta.merge(MediumWeight), color = CoachColors.Meta)
            Text(
                "Skip",
                Modifier.clip(RoundedCornerShape(8.dp)).clickable(role = Role.Button, onClick = onSkip).padding(horizontal = 8.dp, vertical = 4.dp),
                style = type.labelStrong,
                color = CoachColors.Skip,
            )
        }
    }
}
