package app.witness.core.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.traversalIndex
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.Wordmark

@Immutable
data class WebLink(val label: String, val onClick: () -> Unit, val icon: WitnessIcon? = null)

const val CertifiedNote = "Served from an ICP canister · certified response"

enum class WebLayoutMode { Compact, Wide }

@Immutable
data class WebLayoutSpec(val mode: WebLayoutMode, val gutter: Dp, val contentWidth: Dp, val primaryWidth: Dp, val columnGap: Dp)

object WebLayout {
    val WideBreakpoint = 840.dp
    val CompactMaxContent = 560.dp
    val WideMaxContent = 1152.dp
    val WideGutter = 64.dp
    val ColumnGap = 48.dp
    const val PrimaryFraction = 600f / 1152f

    fun spec(width: Dp): WebLayoutSpec {
        val safeWidth = if (width < 0.dp) 0.dp else width
        if (safeWidth < WideBreakpoint) {
            val gutter = 16.dp
            val content = minOf(CompactMaxContent, maxOf(0.dp, safeWidth - gutter * 2))
            return WebLayoutSpec(WebLayoutMode.Compact, gutter, content, content, 0.dp)
        }
        val content = minOf(WideMaxContent, safeWidth - WideGutter * 2)
        val gutter = (safeWidth - content) / 2
        return WebLayoutSpec(WebLayoutMode.Wide, gutter, content, content * PrimaryFraction, ColumnGap)
    }
}

@Composable
fun WebHeader(modifier: Modifier = Modifier, label: String? = "Verification", links: List<WebLink> = emptyList(), wide: Boolean = false) {
    val palette = Witness.colors
    Row(modifier.fillMaxWidth().padding(vertical = if (wide) 24.dp else 14.dp), verticalAlignment = Alignment.CenterVertically) {
        Wordmark(size = if (wide) 22.dp else 19.dp)
        Spacer(Modifier.weight(1f))
        if (links.isNotEmpty()) {
            Row(horizontalArrangement = Arrangement.spacedBy(if (wide) 28.dp else 16.dp)) {
                links.forEach { link -> Text(link.label, Modifier.clickable(role = Role.Button, onClick = link.onClick), style = Witness.type.bodySmallStrong, color = palette.ink2) }
            }
        } else if (label != null) {
            Text(label, style = Witness.type.bodyCompact, color = palette.muted)
        }
    }
}

@Composable
fun WebFooter(links: List<WebLink>, modifier: Modifier = Modifier, wide: Boolean = false, note: String = CertifiedNote) {
    val palette = Witness.colors
    val type = Witness.type
    Column(modifier.fillMaxWidth()) {
        Hairline()
        if (wide) {
            Row(Modifier.fillMaxWidth().padding(top = 16.dp, bottom = 24.dp), verticalAlignment = Alignment.CenterVertically) {
                Row(Modifier.weight(1f)) {
                    links.forEachIndexed { i, link ->
                        if (i > 0) Text(" · ", style = type.labelStrong, color = palette.ink2)
                        Text(link.label, Modifier.clickable(role = Role.Button, onClick = link.onClick), style = type.labelStrong, color = palette.ink2)
                    }
                }
                Text(note, style = type.proofSmall, color = palette.muted)
            }
        } else {
            FlowRow(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                links.forEach { link ->
                    Row(Modifier.clickable(role = Role.Button, onClick = link.onClick).padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (link.icon != null) {
                            WIcon(link.icon, palette.ink2, size = 16.dp)
                            Spacer(Modifier.width(4.dp))
                        }
                        Text(link.label, style = type.captionStrong, color = palette.ink2)
                    }
                }
            }
            Text(note, Modifier.padding(top = 10.dp, bottom = 24.dp), style = type.proofSmall, color = palette.muted)
        }
    }
}

@Composable
fun BrowserSafeLayout(
    modifier: Modifier = Modifier,
    header: @Composable (WebLayoutSpec) -> Unit = {},
    footer: @Composable (WebLayoutSpec) -> Unit = {},
    secondary: (@Composable ColumnScope.(WebLayoutSpec) -> Unit)? = null,
    primary: @Composable ColumnScope.(WebLayoutSpec) -> Unit,
) {
    BoxWithConstraints(modifier.fillMaxSize().background(Witness.colors.paper).windowInsetsPadding(WindowInsets.safeDrawing)) {
        val spec = WebLayout.spec(maxWidth)
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(Modifier.widthIn(max = spec.contentWidth).fillMaxWidth().padding(horizontal = if (spec.mode == WebLayoutMode.Compact) spec.gutter else 0.dp)) {
                Column(Modifier.semantics { traversalIndex = -1f }) { header(spec) }
                when (spec.mode) {
                    WebLayoutMode.Compact -> Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        primary(spec)
                        secondary?.invoke(this, spec)
                    }
                    WebLayoutMode.Wide -> Row(horizontalArrangement = Arrangement.spacedBy(spec.columnGap)) {
                        Column(Modifier.width(spec.primaryWidth), verticalArrangement = Arrangement.spacedBy(16.dp)) { primary(spec) }
                        if (secondary != null) Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(16.dp)) { secondary(spec) }
                    }
                }
                Spacer(Modifier.height(24.dp))
                footer(spec)
            }
        }
    }
}
