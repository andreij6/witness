package app.witness.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.TextAction
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.ToneChip
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes

private val HeroShape = RoundedCornerShape(24.dp)

@Composable
fun PlusRoute(viewModel: PlusViewModel, navigator: SettingsNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LaunchedEffect(viewModel) {
        viewModel.effect.collect {
            when (it) {
                PlusEffect.LinkIdentity -> navigator.navigate(SettingsKey.LinkIdentity)
                PlusEffect.ManageSubscription -> navigator.openExternal(SettingsCopy.PLAY_SUBSCRIPTIONS_URL)
            }
        }
    }
    PlusScreen(
        state = state,
        onClose = navigator::back,
        onSelect = viewModel::select,
        onSubscribe = viewModel::subscribe,
        onManage = viewModel::manage,
        onRestore = viewModel::restore,
        onTerms = { navigator.openExternal(SettingsCopy.TERMS_URL) },
        onLinkIdentity = viewModel::linkIdentity,
        onDismissNotice = viewModel::dismissNotice,
    )
}

@Composable
fun PlusScreen(
    state: PlusUiState,
    onClose: () -> Unit = {},
    onSelect: (PlusPlan) -> Unit = {},
    onSubscribe: () -> Unit = {},
    onManage: () -> Unit = {},
    onRestore: () -> Unit = {},
    onTerms: () -> Unit = {},
    onLinkIdentity: () -> Unit = {},
    onDismissNotice: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    SettingsPage("", onClose, modifier, navigationIcon = WitnessIcon.Close, navigationLabel = "Close") {
        PlusHero(state.active)
        Spacer(Modifier.height(20.dp))
        Row(verticalAlignment = Alignment.Top) {
            WIcon(WitnessIcon.Storage, Witness.colors.ink2, size = 18.dp)
            Spacer(Modifier.width(10.dp))
            Text(
                "25 GB covers hosted full-resolution media. Previews and proof records never count toward it.",
                style = Witness.type.caption.copy(lineHeight = 17.sp),
                color = Witness.colors.ink2,
            )
        }
        Spacer(Modifier.height(24.dp))
        Overline("Choose a plan")
        Spacer(Modifier.height(10.dp))
        Row(Modifier.fillMaxWidth().selectableGroup(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            PlanCard("Monthly", state.monthly.price, "per month", null, state.selected == PlusPlan.MONTHLY, Modifier.weight(1f)) { onSelect(PlusPlan.MONTHLY) }
            PlanCard("Yearly", state.yearly.price, state.yearlyDetail, state.savings, state.selected == PlusPlan.YEARLY, Modifier.weight(1f)) { onSelect(PlusPlan.YEARLY) }
        }
        Spacer(Modifier.height(24.dp))
        state.notice?.let {
            NoticeBanner(it, onDismissNotice)
            Spacer(Modifier.height(12.dp))
        }
        WitnessButton(
            text = when {
                state.active -> SettingsCopy.MANAGE
                state.busy == PlusBusy.Purchasing -> "Opening Google Play…"
                else -> SettingsCopy.SUBSCRIBE
            },
            onClick = if (state.active) onManage else onSubscribe,
            modifier = Modifier.fillMaxWidth(),
            height = 52.dp,
            enabled = state.busy == null,
            textStyle = Witness.type.actionTitle,
        )
        Spacer(Modifier.height(10.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
            TextAction(if (state.busy == PlusBusy.Restoring) "Restoring…" else "Restore purchase", onRestore)
            Text("  ·  ", style = Witness.type.bodyStrong, color = Witness.colors.muted)
            TextAction("Terms", onTerms)
        }
        Spacer(Modifier.height(20.dp))
        IdentityCard(state.account, onLinkIdentity)
        Spacer(Modifier.height(24.dp))
        Text(
            "Proof records are permanent on every plan. Hosted media stays while you're subscribed, plus 30 days to export.",
            Modifier.fillMaxWidth(),
            style = SettingsText.footnote,
            color = Witness.colors.muted,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
private fun PlusHero(active: Boolean) {
    Column(Modifier.fillMaxWidth().clip(HeroShape).background(Witness.colors.ink).padding(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 28.dp)) {
        Row {
            Column(Modifier.weight(1f)) {
                Text("Witness Plus", Modifier.semantics { heading() }, style = Witness.type.verdict, color = WitnessColors.White)
                Spacer(Modifier.height(8.dp))
                Text("Keep full-quality proof online and\nshare it your way.", style = Witness.type.bodySmall.copy(lineHeight = 18.sp), color = OnInkSoft)
            }
            LargeIconDisc(WitnessIcon.WorkspacePremium, WitnessColors.IconCheck, WitnessColors.IconCheck.copy(alpha = 0.12f), 60.dp, 40.dp)
        }
        if (active) {
            Spacer(Modifier.height(12.dp))
            ToneChip("Active", Tone.Ok, icon = WitnessIcon.CheckCircle)
        }
        Spacer(Modifier.height(24.dp))
        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            for (line in PLUS_HERO_FEATURES) CheckBullet(line, Witness.type.body.copy(lineHeight = 18.sp), gap = 10.dp)
        }
    }
}

private val PLUS_HERO_FEATURES = listOf(
    "Full-resolution proof pages · 25 GB",
    "Case reports as PDF",
    "Backup & sync across devices",
    "Creator profile (opt-in)",
)

@Composable
private fun PlanCard(name: String, price: String, detail: String, badge: String?, selected: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val palette = Witness.colors
    Column(
        modifier
            .height(112.dp)
            .clip(WitnessShapes.Card)
            .background(palette.surface)
            .border(if (selected) 2.dp else 1.dp, if (selected) palette.primary else palette.line, WitnessShapes.Card)
            .selectable(selected, role = Role.RadioButton, onClick = onClick)
            .padding(start = 14.dp, end = 12.dp, top = 16.dp, bottom = 12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(if (selected) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked, if (selected) palette.primary else palette.muted, size = 22.dp)
            Spacer(Modifier.width(6.dp))
            Text(name, Modifier.weight(1f), style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1)
            if (badge != null && selected) ToneChip(badge, Tone.Primary, height = 22.dp, fontSize = 10.5.sp)
        }
        Spacer(Modifier.height(10.dp))
        Text(price, Modifier.padding(start = 2.dp), style = Witness.type.pageTitle, color = palette.ink, maxLines = 1)
        Spacer(Modifier.height(4.dp))
        Text(detail, Modifier.padding(start = 2.dp), style = Witness.type.caption, color = palette.ink2, maxLines = 1)
    }
}

@Composable
private fun IdentityCard(account: Principal?, onClick: () -> Unit) {
    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(16.dp), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconDisc(WitnessIcon.Fingerprint, Witness.colors.primary, size = 40.dp, iconSize = 24.dp, fill = Witness.colors.primary.copy(alpha = 0.1f))
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Plus is linked to your Internet Identity", style = Witness.type.bodySmall.copy(fontWeight = FontWeight.Medium), color = Witness.colors.ink)
                Spacer(Modifier.height(2.dp))
                if (account == null) {
                    Text("Link Internet Identity", style = Witness.type.bodyCompact.copy(fontWeight = FontWeight.SemiBold), color = Witness.colors.primary)
                } else {
                    Text("Linked · ${account.short}", style = Witness.type.bodyCompact, color = Witness.colors.ink2)
                }
            }
            Chevron()
        }
    }
}
