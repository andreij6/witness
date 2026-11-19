package app.witness.core.a2ui.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp

data class ChecklistItem(val key: String, val label: String, val done: Boolean, val meta: String?)

data class ChoiceOption(val value: String, val label: String)

private val SurfaceShape = RoundedCornerShape(ReportTokens.SurfaceRadius)

@Composable
fun PlainFrame(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(SurfaceShape)
            .background(ReportTokens.Surface)
            .border(1.dp, ReportTokens.Line, SurfaceShape)
            .padding(start = ReportTokens.Gutter, end = ReportTokens.Gutter, top = 16.dp, bottom = 16.dp),
        content = content,
    )
}

@Composable
fun GeneratedFrame(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(SurfaceShape)
            .background(ReportTokens.Surface)
            .border(1.2.dp, ReportTokens.Ai.copy(alpha = 0.35f), SurfaceShape)
            .drawBehind {
                val inset = ReportTokens.SurfaceRadius.toPx()
                drawRoundRect(
                    color = ReportTokens.Ai.copy(alpha = 0.9f),
                    topLeft = Offset(0f, inset),
                    size = Size(3.dp.toPx(), (size.height - 2 * inset).coerceAtLeast(0f)),
                    cornerRadius = CornerRadius(1.5.dp.toPx()),
                )
            }
            .padding(start = ReportTokens.Gutter, end = ReportTokens.Gutter, top = 16.dp, bottom = 16.dp),
        content = content,
    )
}

@Composable
fun FlagIconButton(onClick: () -> Unit, modifier: Modifier = Modifier) {
    Box(
        modifier
            .size(32.dp)
            .clip(RoundedCornerShape(16.dp))
            .clickable(role = Role.Button, onClickLabel = "Flag", onClick = onClick)
            .semantics { contentDescription = "Flag AI output" },
        contentAlignment = Alignment.Center,
    ) { ReportIconView(ReportIcon.Flag, 20.dp, ReportTokens.Muted) }
}

@Composable
fun AiHeader(label: String, onFlag: (() -> Unit)?, modifier: Modifier = Modifier, trailing: @Composable () -> Unit = {}) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        ReportIconView(ReportIcon.Sparkle, 16.dp, ReportTokens.Ai)
        Spacer(Modifier.width(6.dp))
        BasicText(label.uppercase(), Modifier.weight(1f), style = ReportTokens.Overline.copy(color = ReportTokens.Ai))
        if (onFlag != null) FlagIconButton(onFlag)
        trailing()
    }
}

@Composable
fun AiChip(text: String, modifier: Modifier = Modifier, height: Int = 26) {
    Row(
        modifier
            .height(height.dp)
            .clip(RoundedCornerShape((height / 2).dp))
            .background(ReportTokens.Ai.copy(alpha = 0.12f))
            .padding(horizontal = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ReportIconView(ReportIcon.Sparkle, 13.dp, ReportTokens.Ai)
        Spacer(Modifier.width(5.dp))
        BasicText(text, style = ReportTokens.Chip)
    }
}

@Composable
fun Overline(text: String, modifier: Modifier = Modifier) {
    BasicText(text.uppercase(), modifier, style = ReportTokens.Overline)
}

@Composable
fun EvidenceChecklistCard(
    title: String,
    items: List<ChecklistItem>,
    onCapture: (String) -> Unit,
    onFlag: (() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    GeneratedFrame(modifier) {
        AiHeader(title, onFlag)
        Spacer(Modifier.height(4.dp))
        for (item in items) ChecklistRow(item, onCapture)
    }
}

@Composable
private fun ChecklistRow(item: ChecklistItem, onCapture: (String) -> Unit) {
    Row(Modifier.fillMaxWidth().heightIn(min = 26.dp), verticalAlignment = Alignment.CenterVertically) {
        ReportIconView(
            if (item.done) ReportIcon.CheckCircle else ReportIcon.Add,
            20.dp,
            if (item.done) ReportTokens.Ok else ReportTokens.Primary,
        )
        Spacer(Modifier.width(8.dp))
        BasicText(item.label, Modifier.weight(1f), style = if (item.done) ReportTokens.ItemDone else ReportTokens.ItemOpen)
        when {
            item.done -> item.meta?.let { BasicText(it, style = ReportTokens.Meta) }
            else -> BasicText(
                "Capture",
                Modifier.clickable(role = Role.Button, onClickLabel = "Capture ${item.label}") { onCapture(item.key) },
                style = ReportTokens.Action,
            )
        }
    }
}

@Composable
fun SectionHeading(heading: String, tag: String?, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth().padding(bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        BasicText(heading, Modifier.weight(1f), style = ReportTokens.Heading)
        if (tag != null) AiChip(tag, height = 24)
    }
}

@Composable
fun FieldDivider() {
    Box(Modifier.fillMaxWidth().padding(vertical = 12.dp).height(1.dp).background(ReportTokens.Line))
}

@Composable
fun ImpactDiagramBlock(
    label: String,
    valueLabel: String?,
    caption: String?,
    zone: String?,
    onFlag: (() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    Row(modifier.fillMaxWidth()) {
        Column(Modifier.weight(1f)) {
            Overline(label)
            Spacer(Modifier.height(6.dp))
            BasicText(valueLabel ?: "Not detected", style = ReportTokens.Value)
            if (caption != null) {
                Spacer(Modifier.height(6.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    ReportIconView(ReportIcon.Sparkle, 14.dp, ReportTokens.Ai)
                    Spacer(Modifier.width(4.dp))
                    BasicText(caption, style = ReportTokens.Caption)
                }
            }
        }
        CarDiagram(zone, Modifier.semantics { contentDescription = "Impact diagram: ${valueLabel ?: "none"}" })
        if (onFlag != null) FlagIconButton(onFlag, Modifier.padding(start = 8.dp))
    }
}

@Composable
fun CarDiagram(zone: String?, modifier: Modifier = Modifier) {
    val highlight = ImpactZones.rect(zone)
    Canvas(modifier.size(width = 44.dp, height = 72.dp)) {
        val radius = CornerRadius(14.dp.toPx())
        val body = Path().apply {
            addRoundRect(androidx.compose.ui.geometry.RoundRect(0f, 0f, size.width, size.height, radius))
        }
        drawPath(body, ReportTokens.Surface2)
        val windowInset = 7.dp.toPx()
        drawRoundRect(
            ReportTokens.Surface3,
            Offset(windowInset, 12.dp.toPx()),
            Size(size.width - 2 * windowInset, 14.dp.toPx()),
            CornerRadius(5.dp.toPx()),
        )
        drawRoundRect(
            ReportTokens.Surface3,
            Offset(windowInset, 46.dp.toPx()),
            Size(size.width - 2 * windowInset, 10.dp.toPx()),
            CornerRadius(4.dp.toPx()),
        )
        if (highlight != null) {
            val topLeft = Offset(highlight.left * size.width, highlight.top * size.height)
            val zoneSize = Size((highlight.right - highlight.left) * size.width, (highlight.bottom - highlight.top) * size.height)
            clipPath(body) { drawRect(ReportTokens.Crit.copy(alpha = 0.28f), topLeft, zoneSize) }
            drawCircle(
                ReportTokens.Crit,
                radius = 4.dp.toPx(),
                center = Offset(topLeft.x + zoneSize.width / 2, topLeft.y + zoneSize.height / 2),
            )
        }
        drawPath(body, ReportTokens.Line, style = Stroke(1.dp.toPx()))
    }
}

@Composable
fun PlateFieldBlock(
    label: String,
    value: String?,
    caption: String?,
    onValueChange: ((String) -> Unit)?,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth()) {
        Overline(label)
        Spacer(Modifier.height(8.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier
                    .widthIn(min = 120.dp)
                    .width(180.dp)
                    .height(40.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(ReportTokens.Surface2)
                    .padding(horizontal = 12.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (onValueChange == null) {
                    BasicText(value.orEmpty(), style = ReportTokens.Plate)
                } else {
                    BasicTextField(
                        value = value.orEmpty(),
                        onValueChange = onValueChange,
                        singleLine = true,
                        textStyle = ReportTokens.Plate,
                        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters),
                        modifier = Modifier.fillMaxWidth().semantics { contentDescription = label },
                    )
                }
            }
            if (caption != null) {
                Spacer(Modifier.width(8.dp))
                AiChip(caption)
            }
        }
    }
}

@Composable
fun ChoiceRowBlock(
    label: String,
    options: List<ChoiceOption>,
    selected: String?,
    onSelect: ((String) -> Unit)?,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth()) {
        Overline(label)
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth().selectableGroup(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            for (option in options) {
                val on = option.value == selected
                val shape = RoundedCornerShape(18.dp)
                Box(
                    Modifier
                        .weight(1f)
                        .height(36.dp)
                        .clip(shape)
                        .background(if (on) ReportTokens.Ink else ReportTokens.Surface)
                        .border(1.dp, if (on) ReportTokens.Ink else ReportTokens.Line, shape)
                        .selectable(on, enabled = onSelect != null, role = Role.RadioButton) { onSelect?.invoke(option.value) },
                    contentAlignment = Alignment.Center,
                ) {
                    BasicText(option.label, style = if (on) SelectedChoice else ReportTokens.Choice)
                }
            }
        }
    }
}

private val SelectedChoice: TextStyle = ReportTokens.Choice.copy(
    color = ReportTokens.White,
    fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold,
)

data class SummaryHeader(val label: String? = null, val generated: Boolean = true) {
    fun title(templateTitle: String): String = label ?: templateTitle

    fun placeholder(): String = if (generated) "Nothing drafted yet." else "Nothing written yet."

    fun forTarget(target: String?): SummaryHeader = if (target == SUMMARY_TARGET) this else SummaryHeader()

    companion object {
        const val SUMMARY_TARGET: String = "summary"
    }
}

val LocalSummaryHeader = staticCompositionLocalOf { SummaryHeader() }

@Composable
fun SummaryDraftCard(
    title: String,
    text: String?,
    onEdit: (() -> Unit)?,
    onFlag: (() -> Unit)?,
    modifier: Modifier = Modifier,
    header: SummaryHeader = SummaryHeader(),
) {
    val shown = header.title(title)
    val edit: @Composable () -> Unit = {
        if (onEdit != null) {
            BasicText(
                "Edit",
                Modifier.padding(start = 8.dp).clickable(role = Role.Button, onClickLabel = "Edit $shown", onClick = onEdit),
                style = ReportTokens.Action.copy(fontSize = ReportTokens.Body.fontSize),
            )
        }
    }
    if (header.generated) {
        GeneratedFrame(modifier) {
            AiHeader(shown, onFlag, trailing = edit)
            Spacer(Modifier.height(6.dp))
            BasicText(text ?: header.placeholder(), style = ReportTokens.Body)
        }
    } else {
        PlainFrame(modifier) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Overline(shown, Modifier.weight(1f))
                edit()
            }
            Spacer(Modifier.height(6.dp))
            BasicText(text ?: header.placeholder(), style = ReportTokens.Body)
        }
    }
}
