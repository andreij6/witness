package app.witness.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.SheetHeader
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.WitnessToggle
import app.witness.core.designsystem.outlineChip
import app.witness.core.model.LocationPrecision

private val PlusCardShape = RoundedCornerShape(20.dp)

@Composable
fun SettingsRoute(viewModel: SettingsViewModel, navigator: SettingsNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    SettingsScreen(
        state = state,
        actions = SettingsActions(
            back = navigator::back,
            account = { navigator.navigate(SettingsKey.LinkIdentity) },
            plus = { navigator.navigate(SettingsKey.Plus) },
            manageSubscription = { navigator.openExternal(SettingsCopy.PLAY_SUBSCRIPTIONS_URL) },
            privacyData = { navigator.navigate(SettingsKey.PrivacyData) },
            terms = { navigator.openExternal(SettingsCopy.TERMS_URL) },
            openPrecision = viewModel::openPrecisionPicker,
            closePrecision = viewModel::closePrecisionPicker,
            setPrecision = viewModel::setPrecision,
            setAi = viewModel::setAiEnabled,
            setAnalytics = viewModel::setAnalytics,
            setCrashReports = viewModel::setCrashReports,
            requestDelete = viewModel::requestDelete,
            cancelDelete = viewModel::cancelDelete,
            confirmDelete = viewModel::confirmDelete,
            dismissNotice = viewModel::dismissNotice,
        ),
    )
}

class SettingsActions(
    val back: () -> Unit = {},
    val account: () -> Unit = {},
    val plus: () -> Unit = {},
    val manageSubscription: () -> Unit = {},
    val privacyData: () -> Unit = {},
    val terms: () -> Unit = {},
    val openPrecision: () -> Unit = {},
    val closePrecision: () -> Unit = {},
    val setPrecision: (LocationPrecision) -> Unit = {},
    val setAi: (Boolean) -> Unit = {},
    val setAnalytics: (Boolean) -> Unit = {},
    val setCrashReports: (Boolean) -> Unit = {},
    val requestDelete: () -> Unit = {},
    val cancelDelete: () -> Unit = {},
    val confirmDelete: () -> Unit = {},
    val dismissNotice: () -> Unit = {},
)

@Composable
fun SettingsScreen(state: SettingsUiState, actions: SettingsActions, modifier: Modifier = Modifier) {
    SettingsPage(
        title = "Settings",
        onNavigate = actions.back,
        modifier = modifier,
        overlay = {
            WitnessSheet(state.precisionPicker, actions.closePrecision) { PrecisionPicker(state.precision, actions.setPrecision) }
            if (state.deletion != DeletionState.Hidden) {
                DeleteAccountDialog(running = state.deletion == DeletionState.Running, onCancel = actions.cancelDelete, onConfirm = actions.confirmDelete)
            }
        },
    ) {
        state.notice?.let {
            NoticeBanner(it, actions.dismissNotice)
            Spacer(Modifier.height(12.dp))
        }
        AccountCard(state.account, actions.account)
        Spacer(Modifier.height(16.dp))
        PlusCard(state.plusActive, state.monthlyPrice, if (state.plusActive) actions.manageSubscription else actions.plus)
        Spacer(Modifier.height(12.dp))
        Text(
            "Proof records are permanent on every plan. Hosted media stays while you're subscribed, plus 30 days to export.",
            style = SettingsText.footnote,
            color = Witness.colors.muted,
        )
        Spacer(Modifier.height(24.dp))
        SectionHeader("Privacy") {
            Row(
                Modifier.clip(RoundedCornerShape(8.dp)).clickable(role = Role.Button, onClick = actions.privacyData).padding(vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Privacy & data", style = Witness.type.labelStrong, color = Witness.colors.primary)
                Spacer(Modifier.width(2.dp))
                WIcon(WitnessIcon.ChevronRight, Witness.colors.primary, size = 16.dp)
            }
        }
        ListCard {
            ListRow(WitnessIcon.NearMe, "Location on public posts", subtitle = SettingsCopy.precisionLabel(state.precision), onClick = actions.openPrecision) { Chevron() }
            RowDivider()
            ListRow(WitnessIcon.AutoAwesome, "On-device AI suggestions", subtitle = SettingsCopy.aiSubtitle(state.aiOnDevice)) {
                WitnessToggle(state.aiEnabled, actions.setAi)
            }
            RowDivider()
            ListRow(WitnessIcon.PrivacyTip, "Safety mode", subtitle = "App lock · coming later") {
                WitnessChip("Later", Witness.colors.outlineChip(Witness.colors.muted), height = 22.dp, fontSize = 10.5.sp)
            }
        }
        Spacer(Modifier.height(16.dp))
        SectionHeader("Diagnostics")
        ListCard {
            ListRow(WitnessIcon.TrendingUp, "Analytics", subtitle = "Usage counts · never photos or places") {
                WitnessToggle(state.consent.analytics, actions.setAnalytics)
            }
            RowDivider()
            ListRow(WitnessIcon.Report, "Crash reports", subtitle = "Crash details without record IDs or locations") {
                WitnessToggle(state.consent.crashReports, actions.setCrashReports)
            }
        }
        Spacer(Modifier.height(14.dp))
        ListCard {
            ListRow(WitnessIcon.Description, "Terms, privacy and open-source licenses", tint = Witness.colors.ink, minHeight = 50.dp, onClick = actions.terms)
            RowDivider()
            ListRow(
                WitnessIcon.Delete,
                "Delete account and hosted media",
                tint = Witness.colors.crit,
                titleColor = Witness.colors.crit,
                minHeight = 50.dp,
                onClick = actions.requestDelete,
            )
        }
        Spacer(Modifier.height(14.dp))
        Text("Anonymous proofs stay on-chain after deletion — they contain no personal data.", style = SettingsText.footnote, color = Witness.colors.muted)
    }
}

@Composable
private fun SectionHeader(title: String, trailing: @Composable (() -> Unit)? = null) {
    Row(Modifier.fillMaxWidth().padding(bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Overline(title, Modifier.weight(1f))
        trailing?.invoke()
    }
}

@Composable
private fun AccountCard(principal: Principal?, onClick: () -> Unit) {
    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(16.dp), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconDisc(
                if (principal == null) WitnessIcon.AccountCircle else WitnessIcon.Fingerprint,
                if (principal == null) Witness.colors.ink2 else Witness.colors.primary,
                size = 40.dp,
                iconSize = 24.dp,
                fill = if (principal == null) Witness.colors.surface2 else Witness.colors.primary.copy(alpha = 0.1f),
            )
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text(SettingsCopy.accountTitle(principal), style = Witness.type.actionTitle, color = Witness.colors.ink)
                Spacer(Modifier.height(3.dp))
                Text(SettingsCopy.accountSubtitle(principal), style = Witness.type.caption, color = Witness.colors.ink2)
            }
            Chevron()
        }
    }
}

@Composable
private fun PlusCard(active: Boolean, monthlyPrice: String, onAction: () -> Unit) {
    Column(Modifier.fillMaxWidth().clip(PlusCardShape).background(Witness.colors.ink).padding(18.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.WorkspacePremium, WitnessColors.IconCheck, size = 26.dp)
            Spacer(Modifier.width(8.dp))
            Text("Witness Plus", Modifier.weight(1f).semantics { heading() }, style = Witness.type.qualifierLarge.copy(fontWeight = FontWeight.Bold), color = WitnessColors.White)
            Text(if (active) "Active" else SettingsCopy.monthlyPrice(monthlyPrice), style = Witness.type.bodyStrong, color = if (active) WitnessColors.IconCheck else OnInkSoft)
        }
        Spacer(Modifier.height(16.dp))
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            for (line in PLUS_CARD_FEATURES) CheckBullet(line, Witness.type.bodyCompact.copy(lineHeight = 18.sp))
        }
        Spacer(Modifier.height(20.dp))
        WitnessButton(if (active) SettingsCopy.MANAGE else SettingsCopy.SUBSCRIBE, onAction, Modifier.fillMaxWidth(), height = 40.dp, textStyle = Witness.type.bodyStrong)
    }
}

private val PLUS_CARD_FEATURES = listOf(
    "Full-resolution proof pages · 25 GB included",
    "Case reports as PDF",
    "Backup and sync across devices",
    "Creator profile (opt-in)",
)

@Composable
private fun PrecisionPicker(selected: LocationPrecision, onSelect: (LocationPrecision) -> Unit) {
    SheetHeader("Location on public posts", subtitle = "Exact locations never leave this phone unless you prove them.")
    Column(Modifier.selectableGroup().padding(bottom = 12.dp)) {
        for (p in LocationPrecision.entries) {
            val on = p == selected
            Row(
                Modifier.fillMaxWidth().selectable(on, role = Role.RadioButton) { onSelect(p) }.padding(horizontal = 24.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(SettingsCopy.precisionLabel(p), Modifier.weight(1f), style = SettingsText.rowTitle, color = Witness.colors.ink)
                Box(Modifier.size(22.dp)) {
                    WIcon(if (on) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked, if (on) Witness.colors.primary else Witness.colors.muted, size = 22.dp)
                }
            }
        }
    }
}
