package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt

@Composable
fun WitnessAppBar(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    navigationIcon: WitnessIcon? = WitnessIcon.ArrowBack,
    navigationLabel: String = "Back",
    onNavigate: () -> Unit = {},
    actions: @Composable RowScope.() -> Unit = {},
) {
    val palette = Witness.colors
    Row(
        modifier.fillMaxWidth().height(64.dp).padding(start = if (navigationIcon != null) 8.dp else 16.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (navigationIcon != null) {
            WitnessIconButton(navigationIcon, navigationLabel, onNavigate, tint = palette.ink, size = 40.dp)
            Spacer(Modifier.width(8.dp))
        }
        Column(Modifier.weight(1f)) {
            Text(title, Modifier.semantics { heading() }, style = Witness.type.title, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle != null) Text(subtitle, style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        actions()
    }
}

@Immutable
data class NavItem(val label: String, val icon: WitnessIcon)

internal val NavBarHeight = 80.dp
internal val CaptureRaise = 28.dp
internal val NavCenters = floatArrayOf(0.18f, 0.5f, 0.82f)

internal fun navCenterX(slot: Int, width: Int): Int = (NavCenters[slot] * width).roundToInt()

@Composable
fun WitnessBottomNav(
    start: NavItem,
    end: NavItem,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    onCapture: () -> Unit,
    modifier: Modifier = Modifier,
    captureLabel: String = "Capture",
) {
    val palette = Witness.colors
    Column(modifier.fillMaxWidth()) {
        Layout(
            content = {
                NavTab(start, selectedIndex == 0) { onSelect(0) }
                CaptureButton(captureLabel, onCapture)
                NavTab(end, selectedIndex == 1) { onSelect(1) }
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(NavBarHeight + CaptureRaise)
                .drawBehind {
                    val top = CaptureRaise.toPx()
                    drawRect(palette.surface, Offset(0f, top), Size(size.width, size.height - top))
                    drawRect(palette.line, Offset(0f, top), Size(size.width, 1.dp.toPx()))
                }
                .selectableGroup(),
        ) { measurables, constraints ->
            val loose = Constraints(maxWidth = constraints.maxWidth, maxHeight = constraints.maxHeight)
            val placeables = measurables.map { it.measure(loose) }
            val raise = CaptureRaise.roundToPx()
            val width = constraints.maxWidth
            val height = (NavBarHeight + CaptureRaise).roundToPx()
            layout(width, height) {
                placeables.forEachIndexed { slot, p ->
                    val y = if (slot == 1) 0 else raise
                    p.place(navCenterX(slot, width) - p.width / 2, y)
                }
            }
        }
        Spacer(Modifier.fillMaxWidth().background(palette.surface).windowInsetsPadding(WindowInsets.navigationBars))
    }
}

@Composable
private fun NavTab(item: NavItem, selected: Boolean, onClick: () -> Unit) {
    val palette = Witness.colors
    val color = if (selected) palette.ink else palette.ink2
    Column(
        Modifier
            .size(112.dp, NavBarHeight)
            .selectable(selected = selected, role = Role.Tab, interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onClick)
            .padding(top = 10.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier.size(60.dp, 30.dp).clip(WitnessShapes.Pill).background(if (selected) palette.ink.copy(alpha = 0.08f) else Color.Transparent),
            contentAlignment = Alignment.Center,
        ) { WIcon(item.icon, color) }
        Spacer(Modifier.height(4.dp))
        Text(item.label, style = if (selected) Witness.type.chip else Witness.type.nav, color = color, textAlign = TextAlign.Center, maxLines = 1)
    }
}

@Composable
private fun CaptureButton(label: String, onClick: () -> Unit) {
    val palette = Witness.colors
    Column(Modifier.width(112.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.size(72.dp).clip(CircleShape).background(palette.paper), contentAlignment = Alignment.Center) {
            Box(
                Modifier
                    .size(60.dp)
                    .clip(CircleShape)
                    .background(palette.ink)
                    .clickable(role = Role.Button, onClick = onClick)
                    .semantics { contentDescription = label },
                contentAlignment = Alignment.Center,
            ) { WIcon(WitnessIcon.PhotoCamera, palette.onInk, size = 26.dp) }
        }
        Text(label, style = Witness.type.nav, color = palette.ink2, maxLines = 1)
    }
}
