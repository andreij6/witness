package app.witness.feature.post

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.BottomBar
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.QrCode
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.imageVector

@Composable
fun PostSharedScreen(state: PostSharedUiState, onCopy: (String) -> Unit, onDone: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).statusBarsPadding().padding(bottom = 96.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(Modifier.fillMaxWidth().padding(start = 8.dp, top = 8.dp)) {
                WitnessIconButton(WitnessIcon.Close, "Close", onDone)
            }
            Spacer(Modifier.height(62.dp))
            Box(Modifier.size(112.dp).clip(CircleShape).background(palette.ok.copy(alpha = 0.1f)), contentAlignment = Alignment.Center) {
                Box(Modifier.size(80.dp).clip(CircleShape).background(palette.ok), contentAlignment = Alignment.Center) {
                    Image(WitnessIcon.Verified.imageVector, null, Modifier.size(44.dp), contentScale = ContentScale.Fit, colorFilter = ColorFilter.tint(WitnessColors.White))
                }
            }
            Spacer(Modifier.height(24.dp))
            Text(PostCopy.SHARED_TITLE, Modifier.padding(horizontal = 16.dp).semantics { heading() }, style = Witness.type.pageTitle, color = palette.ink, textAlign = TextAlign.Center)
            Spacer(Modifier.height(8.dp))
            Text(state.subtitle, Modifier.padding(horizontal = 16.dp), style = Witness.type.bodyCompact, color = palette.ink2, textAlign = TextAlign.Center)
            Spacer(Modifier.height(18.dp))
            LinkPill(state.linkText) { onCopy(state.url) }
            Spacer(Modifier.height(28.dp))
            QrCode(state.url, size = 176.dp)
            Spacer(Modifier.height(28.dp))
            Text(PostCopy.SHARED_NOTE, Modifier.padding(horizontal = 16.dp), style = Witness.type.bodyCompact, color = palette.ink2, textAlign = TextAlign.Center)
            Spacer(Modifier.height(24.dp))
            Stats(state)
        }
        BottomBar {
            WitnessButton("Copy link", { onCopy(state.url) }, Modifier.width(140.dp), ButtonKind.Tonal, WitnessIcon.ContentCopy)
            WitnessButton("Done", onDone, Modifier.weight(1f))
        }
    }
}

@Composable
private fun LinkPill(text: String, onCopy: () -> Unit) {
    val primary = Witness.colors.primary
    Row(
        Modifier
            .padding(horizontal = 16.dp)
            .height(40.dp)
            .clip(WitnessShapes.Pill)
            .background(primary.copy(alpha = 0.08f))
            .clickable(role = Role.Button, onClick = onCopy)
            .semantics { contentDescription = "Copy proof link $text" }
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.Link, primary, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Text(text, Modifier.weight(1f, fill = false), style = Witness.type.proof.copy(fontSize = 13.5.sp), color = primary, maxLines = 1, overflow = TextOverflow.MiddleEllipsis)
        Spacer(Modifier.width(14.dp))
        WIcon(WitnessIcon.ContentCopy, primary, size = 18.dp)
    }
}

@Composable
private fun Stats(state: PostSharedUiState) {
    val palette = Witness.colors
    WitnessCard(Modifier.padding(horizontal = 16.dp).fillMaxWidth(), contentPadding = PaddingValues(vertical = 14.dp)) {
        Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.Start) {
            val cells = listOf("Checked" to state.checked, "Shared" to state.shared, "Shows" to state.shows)
            cells.forEachIndexed { i, (label, value) ->
                if (i > 0) Box(Modifier.width(1.dp).fillMaxHeight().background(palette.line))
                Column(Modifier.weight(1f).padding(horizontal = 16.dp)) {
                    Overline(label)
                    Spacer(Modifier.height(6.dp))
                    Text(value, style = Witness.type.actionTitle, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
    }
}
