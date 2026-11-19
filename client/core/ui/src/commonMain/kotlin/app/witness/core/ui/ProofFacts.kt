package app.witness.core.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.ToneChip
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon

enum class FactStatus { Neutral, Passed, Failed }

@Immutable
data class FactBadge(val text: String, val tone: Tone, val icon: WitnessIcon? = null)

@Immutable
data class ProofFact(
    val label: String,
    val value: String,
    val icon: WitnessIcon = WitnessIcon.Info,
    val status: FactStatus = FactStatus.Neutral,
    val badge: FactBadge? = null,
)

@Immutable
data class FactStyle(val icon: WitnessIcon, val iconTone: Tone?, val valueTone: Tone?)

fun ProofFact.style(): FactStyle = when (status) {
    FactStatus.Neutral -> FactStyle(icon, null, null)
    FactStatus.Passed -> FactStyle(WitnessIcon.CheckCircle, Tone.Ok, null)
    FactStatus.Failed -> FactStyle(WitnessIcon.Error, Tone.Crit, Tone.Crit)
}

enum class FactLayout { Inline, Stacked }

@Composable
fun ProofFacts(
    facts: List<ProofFact>,
    modifier: Modifier = Modifier,
    title: String? = null,
    layout: FactLayout = FactLayout.Inline,
    dividers: Boolean = true,
    labelWidth: Dp = 76.dp,
    onFactClick: ((Int) -> Unit)? = null,
) {
    WitnessCard(modifier.fillMaxWidth(), contentPadding = PaddingValues(vertical = 6.dp)) {
        if (title != null) Overline(title, Modifier.padding(start = 16.dp, end = 16.dp, top = 10.dp, bottom = 2.dp))
        facts.forEachIndexed { index, fact ->
            if (index > 0 && dividers) Hairline(Modifier.padding(horizontal = 16.dp))
            FactRow(fact, layout, labelWidth, onFactClick?.let { { it(index) } })
        }
    }
}

@Composable
private fun FactRow(fact: ProofFact, layout: FactLayout, labelWidth: Dp, onClick: (() -> Unit)?) {
    val palette = Witness.colors
    val type = Witness.type
    val style = fact.style()
    val iconColor = style.iconTone?.let(palette::tone) ?: if (layout == FactLayout.Stacked) palette.ink else palette.ink2
    val valueColor = style.valueTone?.let(palette::tone) ?: palette.ink2
    Row(
        Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .semantics(mergeDescendants = true) {}
            .padding(horizontal = 16.dp, vertical = if (layout == FactLayout.Stacked) 12.dp else 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(style.icon, iconColor, size = if (layout == FactLayout.Stacked) 22.dp else 20.dp)
        Spacer(Modifier.width(if (layout == FactLayout.Stacked) 14.dp else 10.dp))
        when (layout) {
            FactLayout.Inline -> {
                Text(fact.label, Modifier.width(labelWidth), style = type.bodySmallStrong, color = palette.ink, maxLines = 1)
                Text(fact.value, Modifier.weight(1f), style = type.subtitle, color = valueColor)
            }
            FactLayout.Stacked -> Column(Modifier.weight(1f)) {
                Text(fact.label, style = type.bodyStrong, color = palette.ink)
                Spacer(Modifier.height(2.dp))
                Text(fact.value, style = type.caption, color = valueColor)
            }
        }
        if (fact.badge != null) ToneChip(fact.badge.text, fact.badge.tone, icon = fact.badge.icon, fontSize = 11.5.sp)
        if (onClick != null) WIcon(WitnessIcon.ChevronRight, palette.muted, size = 22.dp)
    }
}
