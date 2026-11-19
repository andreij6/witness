package app.witness.feature.settings

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.TonePanel
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.imageVector

internal val OnInkStrong = WitnessColors.White.copy(alpha = 0.9f)
internal val OnInkSoft = WitnessColors.White.copy(alpha = 0.77f)

internal object SettingsText {
    val rowTitle: TextStyle
        @Composable @ReadOnlyComposable get() = Witness.type.body.copy(fontWeight = FontWeight.Medium, lineHeight = 18.sp)
    val rowSubtitle: TextStyle
        @Composable @ReadOnlyComposable get() = Witness.type.caption
    val footnote: TextStyle
        @Composable @ReadOnlyComposable get() = Witness.type.meta.copy(lineHeight = 16.sp)
}

@Composable
internal fun SettingsPage(
    title: String,
    onNavigate: () -> Unit,
    modifier: Modifier = Modifier,
    navigationIcon: WitnessIcon = WitnessIcon.ArrowBack,
    navigationLabel: String = "Back",
    overlay: @Composable () -> Unit = {},
    content: @Composable ColumnScope.() -> Unit,
) {
    Box(modifier.fillMaxSize().background(Witness.colors.paper)) {
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
            WitnessAppBar(title, navigationIcon = navigationIcon, navigationLabel = navigationLabel, onNavigate = onNavigate)
            Column(
                Modifier.fillMaxWidth().weight(1f).verticalScroll(rememberScrollState()).padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 24.dp),
                content = content,
            )
        }
        overlay()
    }
}

@Composable
internal fun ListCard(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) =
    WitnessCard(modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp), content = content)

@Composable
internal fun ListRow(
    icon: WitnessIcon,
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    tint: Color = Witness.colors.ink2,
    titleColor: Color = Witness.colors.ink,
    titleStyle: TextStyle = SettingsText.rowTitle,
    minHeight: Dp = 56.dp,
    role: Role = Role.Button,
    onClick: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = minHeight)
            .then(if (onClick != null) Modifier.clickable(role = role, onClick = onClick) else Modifier)
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(icon, tint, size = 22.dp)
        Spacer(Modifier.width(14.dp))
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = titleStyle, color = titleColor)
            if (subtitle != null) Text(subtitle, style = SettingsText.rowSubtitle, color = Witness.colors.ink2)
        }
        if (trailing != null) {
            Spacer(Modifier.width(12.dp))
            trailing()
        }
    }
}

@Composable
internal fun Chevron(tint: Color = Witness.colors.muted) = WIcon(WitnessIcon.ChevronRight, tint)

@Composable
internal fun RowDivider(start: Dp = 16.dp) = Hairline(Modifier.padding(start = start, end = 16.dp))

@Composable
internal fun CheckBullet(text: String, style: TextStyle, color: Color = OnInkStrong, gap: Dp = 8.dp) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        WIcon(WitnessIcon.Check, WitnessColors.IconCheck, size = 18.dp)
        Spacer(Modifier.width(gap))
        Text(text, style = style, color = color)
    }
}

@Composable
internal fun DotRow(text: String, modifier: Modifier = Modifier) {
    Row(modifier) {
        Box(Modifier.padding(top = 6.dp, start = 1.dp).size(5.dp).clip(CircleShape).background(Witness.colors.ink2))
        Spacer(Modifier.width(9.dp))
        Text(text, style = Witness.type.bodySmall, color = Witness.colors.ink)
    }
}

@Composable
internal fun NoticeBanner(notice: Notice, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    val color = Witness.colors.tone(notice.tone)
    TonePanel(color, modifier.fillMaxWidth(), contentPadding = PaddingValues(start = 16.dp, top = 6.dp, bottom = 6.dp, end = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(notice.text, Modifier.weight(1f), style = Witness.type.bodySmall, color = Witness.colors.ink)
            WitnessIconButton(WitnessIcon.Close, "Dismiss", onDismiss, tint = Witness.colors.ink2, iconSize = 20.dp)
        }
    }
}

@Composable
internal fun LargeIconDisc(icon: WitnessIcon, tint: Color, fill: Color, size: Dp, iconSize: Dp, modifier: Modifier = Modifier) {
    Box(modifier.size(size).clip(CircleShape).background(fill), contentAlignment = Alignment.Center) {
        Image(icon.imageVector, null, Modifier.size(iconSize), colorFilter = ColorFilter.tint(tint))
    }
}
