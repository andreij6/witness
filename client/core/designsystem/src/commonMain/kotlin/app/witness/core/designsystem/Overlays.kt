package app.witness.core.designsystem

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog

const val ScrimAlpha = 0.45f

@Composable
fun SheetHandle(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().padding(top = 8.dp, bottom = 8.dp), contentAlignment = Alignment.Center) {
        Box(Modifier.size(36.dp, 4.dp).clip(WitnessShapes.Pill).background(Witness.colors.muted.copy(alpha = 0.5f)))
    }
}

@Composable
fun SheetSurface(modifier: Modifier = Modifier, color: Color = Witness.colors.paper, handle: Boolean = true, content: @Composable ColumnScope.() -> Unit) {
    Column(modifier.fillMaxWidth().clip(WitnessShapes.Sheet).background(color)) {
        if (handle) SheetHandle()
        content()
    }
}

@Composable
fun Scrim(onDismiss: () -> Unit, modifier: Modifier = Modifier, dismissLabel: String = "Dismiss") {
    Box(
        modifier
            .fillMaxSize()
            .background(Witness.colors.scrim.copy(alpha = ScrimAlpha))
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onDismiss)
            .semantics { contentDescription = dismissLabel },
    )
}

@Composable
fun WitnessSheet(visible: Boolean, onDismiss: () -> Unit, modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Box(modifier.fillMaxSize()) {
        AnimatedVisibility(visible, enter = fadeIn(), exit = fadeOut()) { Scrim(onDismiss) }
        AnimatedVisibility(
            visible,
            modifier = Modifier.align(Alignment.BottomCenter),
            enter = slideInVertically { it },
            exit = slideOutVertically { it },
        ) {
            SheetSurface(color = Witness.colors.surface) {
                content()
                Spacer(Modifier.windowInsetsPadding(WindowInsets.navigationBars))
            }
        }
    }
}

@Composable
fun SheetHeader(title: String, modifier: Modifier = Modifier, subtitle: String? = null) {
    Column(modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, top = 12.dp, bottom = 16.dp)) {
        Text(title, Modifier.semantics { heading() }, style = Witness.type.section, color = Witness.colors.ink)
        if (subtitle != null) Text(subtitle, style = Witness.type.subtitle, color = Witness.colors.ink2)
    }
}

@Composable
fun WitnessDialog(
    title: String,
    onDismiss: () -> Unit,
    confirmLabel: String,
    onConfirm: () -> Unit,
    modifier: Modifier = Modifier,
    text: String? = null,
    dismissLabel: String? = "Cancel",
    icon: WitnessIcon? = null,
    destructive: Boolean = false,
    content: @Composable ColumnScope.() -> Unit = {},
) {
    Dialog(onDismissRequest = onDismiss) {
        DialogCard(title, onDismiss, confirmLabel, onConfirm, modifier, text, dismissLabel, icon, destructive, content)
    }
}

@Composable
fun DialogCard(
    title: String,
    onDismiss: () -> Unit,
    confirmLabel: String,
    onConfirm: () -> Unit,
    modifier: Modifier = Modifier,
    text: String? = null,
    dismissLabel: String? = "Cancel",
    icon: WitnessIcon? = null,
    destructive: Boolean = false,
    content: @Composable ColumnScope.() -> Unit = {},
) {
    val palette = Witness.colors
    val accent = if (destructive) palette.crit else palette.ink
    Column(
        modifier.widthIn(max = 380.dp).clip(WitnessShapes.Dialog).background(palette.surface).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (icon != null) IconDisc(icon, accent, size = 48.dp, iconSize = 24.dp)
        Text(title, Modifier.semantics { heading() }, style = Witness.type.title, color = palette.ink)
        if (text != null) Text(text, style = Witness.type.body, color = palette.ink2)
        content()
        Spacer(Modifier.height(4.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            if (dismissLabel != null) WitnessButton(dismissLabel, onDismiss, Modifier.weight(1f), ButtonKind.Outlined)
            WitnessButton(confirmLabel, onConfirm, Modifier.weight(1f), if (destructive) ButtonKind.Danger else ButtonKind.Filled)
        }
    }
}

@Composable
fun BoxScope.BottomBar(modifier: Modifier = Modifier, content: @Composable RowScope.() -> Unit) {
    Column(modifier.align(Alignment.BottomCenter).fillMaxWidth().background(Witness.colors.surface)) {
        Hairline()
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            content = content,
        )
        Spacer(Modifier.windowInsetsPadding(WindowInsets.navigationBars))
    }
}
