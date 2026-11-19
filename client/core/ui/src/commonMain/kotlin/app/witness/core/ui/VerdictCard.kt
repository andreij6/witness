package app.witness.core.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.TonePanel
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.model.Verdict

@Composable
fun VerdictCard(
    verdict: Verdict,
    modifier: Modifier = Modifier,
    headline: String = verdict.headline(),
    qualifier: String? = verdict.qualifier(),
    body: String? = verdict.summary(),
    large: Boolean = verdict == Verdict.ALTERED,
) {
    val colors = verdict.colors(Witness.colors)
    val type = Witness.type
    TonePanel(
        colors.icon,
        modifier.fillMaxWidth().semantics(mergeDescendants = true) { liveRegion = LiveRegionMode.Polite },
        fill = colors.fill,
        stroke = colors.stroke,
        contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 18.dp),
    ) {
        Row(verticalAlignment = if (qualifier == null) Alignment.CenterVertically else Alignment.Top) {
            WIcon(verdict.state().icon, colors.icon, size = if (large) 34.dp else 30.dp)
            Spacer(Modifier.width(if (large) 12.dp else 10.dp))
            Column {
                Text(headline, Modifier.semantics { heading() }, style = when {
                    large -> type.verdict
                    verdict == Verdict.NO_RECORD -> type.headline
                    else -> type.verdictCompact
                }, color = colors.title)
                if (qualifier != null) Text(qualifier, style = type.bodyStrong, color = colors.title)
            }
        }
        if (body != null) {
            Spacer(Modifier.height(12.dp))
            Text(body, style = if (large || verdict == Verdict.NO_RECORD) type.bodySmall else type.bodyCompact, color = Witness.colors.ink)
        }
    }
}

@Composable
fun VerdictHeadline(verdict: Verdict, modifier: Modifier = Modifier, qualifier: String? = verdict.qualifier(), note: String? = null, large: Boolean = false) {
    val colors = verdict.colors(Witness.colors)
    val type = Witness.type
    Column(modifier.semantics(mergeDescendants = true) {}) {
        Text(verdict.headline(), Modifier.semantics { heading() }, style = if (large) type.hero else type.pageTitle, color = colors.title)
        FlowRow(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            if (qualifier != null) Text(qualifier, style = if (large) type.qualifierLarge else type.bodyStrong, color = colors.title)
            if (note != null) Text("· $note", style = if (large) type.bodyLarge else type.bodySmall, color = Witness.colors.ink2)
        }
    }
}
