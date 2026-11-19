package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessTheme

object UnsupportedCopy {
    const val TITLE = "This phone can't seal captures"
    const val LEAD = "Witness couldn't confirm this phone's security. Usually the bootloader is unlocked or the device can't prove it, so photos taken here can't be proven original."
    const val REASONS = "Common reasons"
    val reasons = listOf("Bootloader is unlocked", "Rooted or custom system software", "No hardware key store on this model", "Security patch is too old")
    const val CHECK_TITLE = "You can still check any photo"
    const val CHECK_DETAIL = "See if a photo is an original, matches one with changes, or has no Witness record."
    const val CHECK = "Check a photo"
    const val LEARN_WHY = "Learn why"
    const val LEARN_WHY_URL = "https://witness.app/help/unsupported-device"
}

@Composable
fun UnsupportedScreen(onCheckPhoto: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val uri = LocalUriHandler.current
    Column(
        modifier
            .fillMaxSize()
            .background(palette.paper)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = Gutter),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        VSpace(112.dp)
        IconDisc(WitnessIcon.GppBad, palette.crit, size = 96.dp, iconSize = 48.dp)
        VSpace(28.dp)
        ScreenTitle(UnsupportedCopy.TITLE, textAlign = TextAlign.Center, style = Witness.type.headline)
        VSpace(12.dp)
        Lead(UnsupportedCopy.LEAD, Modifier.padding(horizontal = 8.dp), textAlign = TextAlign.Center)
        VSpace(24.dp)
        WitnessCard(Modifier.fillMaxWidth()) {
            Overline(UnsupportedCopy.REASONS)
            UnsupportedCopy.reasons.forEach { reason ->
                VSpace(10.dp)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.padding(start = 2.dp).size(10.dp, 2.dp).background(palette.muted))
                    HSpace(14.dp)
                    Text(reason, style = Witness.type.bodySmall, color = palette.ink)
                }
            }
        }
        VSpace(16.dp)
        WitnessCard(Modifier.fillMaxWidth()) {
            Row {
                IconDisc(WitnessIcon.VerifiedUser, palette.primary, size = 44.dp, iconSize = 24.dp)
                HSpace(16.dp)
                Column(Modifier.weight(1f)) {
                    Text(UnsupportedCopy.CHECK_TITLE, style = Witness.type.cardTitle, color = palette.ink)
                    VSpace(4.dp)
                    Text(UnsupportedCopy.CHECK_DETAIL, style = Witness.type.bodyCompact, color = palette.ink2)
                }
            }
            VSpace(20.dp)
            WitnessButton(UnsupportedCopy.CHECK, onCheckPhoto, Modifier.fillMaxWidth(), icon = WitnessIcon.VerifiedUser)
        }
        VSpace(24.dp)
        Row(
            Modifier.clip(WitnessShapes.Field).clickable(role = Role.Button) { uri.openUri(UnsupportedCopy.LEARN_WHY_URL) }.padding(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(UnsupportedCopy.LEARN_WHY, style = Witness.type.bodyStrong, color = palette.primary)
            HSpace(6.dp)
            WIcon(WitnessIcon.OpenInNew, palette.primary, size = 16.dp)
        }
        VSpace(24.dp)
    }
}

@Preview
@Composable
private fun UnsupportedPreview() = WitnessTheme { UnsupportedScreen({}) }
