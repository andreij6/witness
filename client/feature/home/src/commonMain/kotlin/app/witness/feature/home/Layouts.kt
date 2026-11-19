package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Witness

internal val Gutter = 16.dp
internal val LargeButtonHeight = 52.dp

@Composable
internal fun OnboardingLayout(
    modifier: Modifier = Modifier,
    footer: @Composable ColumnScope.() -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(modifier.fillMaxSize().background(Witness.colors.paper).windowInsetsPadding(WindowInsets.safeDrawing)) {
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Gutter), content = content)
        Column(
            Modifier.fillMaxWidth().padding(start = Gutter, end = Gutter, top = 16.dp, bottom = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            content = footer,
        )
    }
}

@Composable
internal fun PageDots(count: Int, selected: Int, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Row(
        modifier.semantics { contentDescription = "Step ${selected + 1} of $count" },
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        repeat(count) { i ->
            Box(
                Modifier.size(if (i == selected) 22.dp else 8.dp, 8.dp)
                    .clip(RoundedCornerShape(4.dp))
                    .background(if (i == selected) palette.ink else palette.surface3),
            )
        }
    }
}

@Composable
internal fun ScreenTitle(text: String, modifier: Modifier = Modifier, textAlign: TextAlign? = null, style: TextStyle = Witness.type.display) {
    Text(text, modifier.fillMaxWidth().semantics { heading() }, style = style, color = Witness.colors.ink, textAlign = textAlign)
}

@Composable
internal fun Lead(text: String, modifier: Modifier = Modifier, textAlign: TextAlign? = null) {
    Text(text, modifier.fillMaxWidth(), style = Witness.type.body.merge(LeadStyle), color = Witness.colors.ink2, textAlign = textAlign)
}

@Composable
internal fun Footnote(text: String, modifier: Modifier = Modifier, color: Color = Witness.colors.muted) {
    Text(text, modifier.fillMaxWidth(), style = Witness.type.caption, color = color, textAlign = TextAlign.Center)
}

@Composable
internal fun VSpace(height: Dp) = Spacer(Modifier.height(height))

@Composable
internal fun HSpace(width: Dp) = Spacer(Modifier.width(width))

internal val LeadStyle = TextStyle(lineHeight = 1.45.em)
internal val LargeButtonText = TextStyle(fontSize = 15.sp)
