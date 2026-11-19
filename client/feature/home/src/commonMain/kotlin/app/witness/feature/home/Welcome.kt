package app.witness.feature.home

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.GlassPill
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.SealBadge
import app.witness.core.designsystem.TextAction
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.designsystem.solidChip
import app.witness.feature.home.generated.Res
import app.witness.feature.home.generated.welcome_hail
import org.jetbrains.compose.resources.painterResource

@Immutable
data class Promise(val icon: WitnessIcon, val text: String)

object WelcomeCopy {
    const val TITLE = "Proof, not just pictures"
    const val LEAD = "Witness seals each photo on your phone the moment you take it, so anyone can check it later."
    const val GET_STARTED = "Get started"
    const val CHECK_ONLY = "I just want to check a photo"
    const val SAMPLE_PLACE = "Denver area · Nov 18, 4:12 PM"
    val promises = listOf(
        Promise(WitnessIcon.Lock, "Sealed by your phone's security chip"),
        Promise(WitnessIcon.TrendingUp, "Proof survives screenshots and reposts"),
        Promise(WitnessIcon.VisibilityOff, "Your photos stay on your phone"),
    )
}

@Composable
fun WelcomeScreen(onGetStarted: () -> Unit, onCheckPhoto: () -> Unit, modifier: Modifier = Modifier) {
    OnboardingLayout(
        modifier,
        footer = {
            PageDots(3, 0)
            VSpace(20.dp)
            WitnessButton(WelcomeCopy.GET_STARTED, onGetStarted, Modifier.fillMaxWidth(), height = LargeButtonHeight, textStyle = Witness.type.button.merge(LargeButtonText))
            VSpace(12.dp)
            TextAction(WelcomeCopy.CHECK_ONLY, onCheckPhoto)
        },
    ) {
        VSpace(32.dp)
        WelcomeHero()
        VSpace(24.dp)
        ScreenTitle(WelcomeCopy.TITLE)
        VSpace(10.dp)
        Lead(WelcomeCopy.LEAD)
        VSpace(14.dp)
        WelcomeCopy.promises.forEach { p ->
            VSpace(12.dp)
            Row(verticalAlignment = Alignment.CenterVertically) {
                IconDisc(p.icon, Witness.colors.ink, size = 40.dp, iconSize = 22.dp, fill = Witness.colors.surface3)
                HSpace(16.dp)
                Text(p.text, style = Witness.type.label.copy(fontSize = 14.5.sp), color = Witness.colors.ink)
            }
        }
        VSpace(12.dp)
    }
}

@Immutable
data class Redaction(val cx: Float, val cy: Float, val w: Float, val h: Float)

internal val heroRedactions = listOf(Redaction(0.72f, 0.366f, 0.040f, 0.075f), Redaction(0.567f, 0.606f, 0.060f, 0.035f))
private val mosaicShades = listOf(Color(0xFF5E6064), Color(0xFFA7A9AC), Color(0xFF47494D), Color(0xFFC4C5C7), Color(0xFF7F8185))
internal const val MOSAIC_CELLS = 4

internal fun mosaicShade(region: Int, i: Int, j: Int): Int = (region * 7 + i * 3 + j * 5 + i * j) % 5

@Composable
private fun WelcomeHero() {
    Box(Modifier.fillMaxWidth().height(360.dp).clip(RoundedCornerShape(24.dp)).semantics(mergeDescendants = true) {}) {
        Image(painterResource(Res.drawable.welcome_hail), null, Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        Box(Modifier.matchParentSize().drawBehind { drawRedactions() })
        WitnessChip(
            "Sealed",
            Witness.colors.solidChip(Witness.colors.ink),
            Modifier.padding(14.dp),
            icon = WitnessIcon.Lock,
            height = 28.dp,
            fontSize = 12.5.sp,
        )
        GlassPill(WelcomeCopy.SAMPLE_PLACE, WitnessIcon.LocationOn, Modifier.align(Alignment.BottomStart).padding(14.dp))
        SealBadge(WitnessState.Anchored, Modifier.align(Alignment.BottomEnd).padding(12.dp), radius = 16.dp)
    }
}

private fun DrawScope.drawRedactions() {
    heroRedactions.forEachIndexed { r, red ->
        val w = red.w * size.width
        val h = red.h * size.height
        val left = red.cx * size.width - w / 2
        val top = red.cy * size.height - h / 2
        val cw = w / MOSAIC_CELLS
        val ch = h / MOSAIC_CELLS
        for (i in 0 until MOSAIC_CELLS) for (j in 0 until MOSAIC_CELLS) {
            drawRect(mosaicShades[mosaicShade(r, i, j)], Offset(left + i * cw, top + j * ch), Size(cw + 0.5f, ch + 0.5f))
        }
    }
}

@Preview
@Composable
private fun WelcomePreview() = WitnessTheme { WelcomeScreen({}, {}) }
