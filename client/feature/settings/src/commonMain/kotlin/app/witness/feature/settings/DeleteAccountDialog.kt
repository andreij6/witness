package app.witness.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessIcon

private val DialogShape = RoundedCornerShape(24.dp)
private val NoteShape = RoundedCornerShape(14.dp)

object DeleteAccountCopy {
    const val TITLE = "Delete account and hosted media?"
    val DELETED = listOf("Previews and proof pages", "Encrypted case links", "Fingerprints", "Your Internet Identity link")
    val STAYS = listOf("Anonymous anchored proofs — no images or personal data")
    const val NOTE = "Cancel Witness Plus in Google Play separately."
}

@Composable
fun DeleteAccountDialog(running: Boolean, onCancel: () -> Unit, onConfirm: () -> Unit) {
    Dialog(
        onDismissRequest = { if (!running) onCancel() },
        properties = DialogProperties(dismissOnBackPress = !running, dismissOnClickOutside = !running, usePlatformDefaultWidth = false),
    ) {
        DeleteAccountCard(running, onCancel, onConfirm, Modifier.padding(horizontal = 24.dp))
    }
}

@Composable
fun DeleteAccountCard(running: Boolean, onCancel: () -> Unit, onConfirm: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Column(
        modifier.fillMaxWidth().clip(DialogShape).background(palette.surface).verticalScroll(rememberScrollState()).padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        LargeIconDisc(WitnessIcon.Delete, palette.crit, palette.crit.copy(alpha = 0.12f), 48.dp, 28.dp)
        Spacer(Modifier.height(16.dp))
        Text(
            DeleteAccountCopy.TITLE,
            Modifier.fillMaxWidth().semantics { heading() },
            style = Witness.type.title,
            color = palette.ink,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(20.dp))
        ListSection(WitnessIcon.Delete, "Deleted", palette.crit, DeleteAccountCopy.DELETED)
        Spacer(Modifier.height(20.dp))
        ListSection(WitnessIcon.Lock, "Stays", palette.ink, DeleteAccountCopy.STAYS)
        Spacer(Modifier.height(24.dp))
        Row(
            Modifier.fillMaxWidth().clip(NoteShape).background(palette.surface2).padding(horizontal = 12.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.Info, palette.ink2, size = 20.dp)
            Spacer(Modifier.width(10.dp))
            Text(DeleteAccountCopy.NOTE, style = Witness.type.subtitle, color = palette.ink)
        }
        Spacer(Modifier.height(28.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            WitnessButton("Cancel", onCancel, Modifier.weight(1f), ButtonKind.Outlined, enabled = !running)
            WitnessButton(if (running) "Deleting…" else "Delete", onConfirm, Modifier.weight(1f), ButtonKind.Danger, enabled = !running)
        }
    }
}

@Composable
private fun ListSection(icon: WitnessIcon, title: String, color: Color, items: List<String>) {
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(Modifier.padding(bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            WIcon(icon, color, size = 18.dp)
            Spacer(Modifier.width(8.dp))
            Overline(title, color = color)
        }
        for (item in items) DotRow(item)
    }
}
