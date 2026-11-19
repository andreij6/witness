package app.witness.feature.settings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon

private val NoteShape = RoundedCornerShape(18.dp)

@Composable
fun IdentityLinkRoute(viewModel: IdentityLinkViewModel, navigator: SettingsNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LaunchedEffect(viewModel) {
        viewModel.effect.collect {
            when (it) {
                is IdentityEffect.OpenBrowser -> navigator.openExternal(it.url)
                IdentityEffect.Close -> navigator.back()
            }
        }
    }
    IdentityLinkScreen(state, onBack = navigator::back, onContinue = viewModel::start, onNotNow = viewModel::notNow, onUnlink = viewModel::unlink, onDismissError = viewModel::dismissError)
}

@Composable
fun IdentityLinkScreen(
    state: IdentityLinkUiState,
    onBack: () -> Unit = {},
    onContinue: () -> Unit = {},
    onNotNow: () -> Unit = {},
    onUnlink: () -> Unit = {},
    onDismissError: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val palette = Witness.colors
    SettingsPage("Link Internet Identity", onBack, modifier) {
        Spacer(Modifier.height(36.dp))
        IdentityIllustration()
        Spacer(Modifier.height(28.dp))
        Text(
            if (state.linked == null) "One identity for all your phones" else "Internet Identity linked",
            Modifier.fillMaxWidth().semantics { heading() },
            style = Witness.type.headline,
            color = palette.ink,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(12.dp))
        Text(
            state.linked?.let { "Linked as ${it.short}. Only you and the registry can see this link." }
                ?: "Internet Identity uses your phone's passkey. No email, no password.",
            Modifier.fillMaxWidth().padding(horizontal = 24.dp),
            style = Witness.type.body.copy(lineHeight = 20.sp),
            color = palette.ink2,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(32.dp))
        ListCard {
            BENEFITS.forEachIndexed { i, (icon, text) ->
                if (i > 0) RowDivider()
                Row(Modifier.fillMaxWidth().height(64.dp).padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                    IconDisc(icon, palette.primary, size = 36.dp, iconSize = 21.dp, fill = palette.primary.copy(alpha = 0.1f))
                    Spacer(Modifier.width(14.dp))
                    Text(text, style = Witness.type.bodySmall.copy(fontWeight = FontWeight.Medium), color = palette.ink)
                }
            }
        }
        Spacer(Modifier.height(20.dp))
        Row(
            Modifier.fillMaxWidth().clip(NoteShape).background(palette.surface2).padding(horizontal = 16.dp, vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.VisibilityOff, palette.ink2, size = 22.dp)
            Spacer(Modifier.width(12.dp))
            Text("Your proofs stay anonymous — the link is never public.", style = Witness.type.bodyCompact.copy(lineHeight = 18.sp), color = palette.ink)
        }
        Spacer(Modifier.height(24.dp))
        Text("Optional. Witness works fully without an account.", Modifier.fillMaxWidth(), style = Witness.type.caption, color = palette.muted, textAlign = TextAlign.Center)
        state.error?.let {
            Spacer(Modifier.height(16.dp))
            NoticeBanner(Notice(it, Tone.Crit), onDismissError)
        }
        Spacer(Modifier.height(40.dp))
        if (state.linked == null) {
            WitnessButton(
                if (state.waiting) "Waiting for Internet Identity…" else "Continue with Internet Identity",
                onContinue,
                Modifier.fillMaxWidth(),
                icon = WitnessIcon.Login,
                height = 52.dp,
                enabled = !state.waiting,
                textStyle = Witness.type.actionTitle,
            )
            Spacer(Modifier.height(12.dp))
            WitnessButton("Not now", onNotNow, Modifier.fillMaxWidth(), ButtonKind.Outlined)
        } else {
            WitnessButton("Done", onNotNow, Modifier.fillMaxWidth(), height = 52.dp, textStyle = Witness.type.actionTitle)
            Spacer(Modifier.height(12.dp))
            WitnessButton("Unlink", onUnlink, Modifier.fillMaxWidth(), ButtonKind.Outlined)
        }
    }
}

private val BENEFITS = listOf(
    WitnessIcon.Delete to "Delete your posts from any device or the web",
    WitnessIcon.Sync to "Sync your captures across phones",
    WitnessIcon.WorkspacePremium to "Required for Witness Plus",
)

@Composable
private fun IdentityIllustration() {
    val palette = Witness.colors
    Box(Modifier.fillMaxWidth().height(120.dp), contentAlignment = Alignment.Center) {
        Canvas(Modifier.width(192.dp).height(2.dp)) {
            drawLine(palette.line, Offset(0f, 0f), Offset(size.width, 0f), 2.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 6.dp.toPx())))
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            LargeIconDisc(WitnessIcon.Key, palette.ink2, palette.surface2, 60.dp, 36.dp)
            Spacer(Modifier.width(14.dp))
            Box(Modifier.size(120.dp).clip(CircleShape).background(palette.paper), contentAlignment = Alignment.Center) {
                Box(Modifier.size(112.dp).clip(CircleShape).background(palette.primary.copy(alpha = 0.1f)), contentAlignment = Alignment.Center) {
                    LargeIconDisc(WitnessIcon.Fingerprint, WitnessColors.White, palette.primary, 80.dp, 48.dp)
                }
            }
            Spacer(Modifier.width(14.dp))
            LargeIconDisc(WitnessIcon.Devices, palette.ink2, palette.surface2, 60.dp, 36.dp)
        }
    }
}
