package app.witness.core.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon

@Immutable
data class ProvesLine(val text: String, val proven: Boolean)

object Proves {
    val Default = listOf(
        ProvesLine("Genuine phone running the real Witness app", true),
        ProvesLine("Existed no later than its anchor time", true),
        ProvesLine("Unaltered, except declared edits", true),
        ProvesLine("Doesn't prove the scene wasn't staged", false),
    )

    val Dev = listOf(
        ProvesLine("Development device, not a verified phone", false),
        ProvesLine("Existed no later than its anchor time", true),
        ProvesLine("Unaltered, except declared edits", true),
        ProvesLine("Doesn't prove the scene wasn't staged", false),
    )

    val CaseLink = listOf(
        ProvesLine("Genuine phone, real Witness app", true),
        ProvesLine("Unaltered, except declared edits", true),
        ProvesLine("Doesn't prove the scene wasn't staged", false),
    )

    val CaseLinkDev = listOf(
        ProvesLine("Development device, not a verified phone", false),
        ProvesLine("Unaltered, except declared edits", true),
        ProvesLine("Doesn't prove the scene wasn't staged", false),
    )

    fun accessibleText(line: ProvesLine): String = if (line.proven) "Proves: ${line.text}" else line.text
}

@Composable
fun ProvesCard(modifier: Modifier = Modifier, lines: List<ProvesLine> = Proves.Default, title: String = "What this proves") {
    val palette = Witness.colors
    WitnessCard(modifier.fillMaxWidth()) {
        Overline(title)
        Spacer(Modifier.height(8.dp))
        lines.forEach { line ->
            Row(
                Modifier.fillMaxWidth().padding(vertical = 3.dp).semantics(mergeDescendants = true) { contentDescription = Proves.accessibleText(line) },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.size(18.dp), contentAlignment = Alignment.Center) {
                    if (line.proven) WIcon(WitnessIcon.Check, palette.ok, size = 18.dp)
                    else Box(Modifier.size(10.dp, 2.dp).background(palette.muted))
                }
                Spacer(Modifier.width(8.dp))
                Text(line.text, style = Witness.type.subtitle, color = if (line.proven) palette.ink else palette.muted)
            }
        }
    }
}
