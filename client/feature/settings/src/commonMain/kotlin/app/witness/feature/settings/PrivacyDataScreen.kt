package app.witness.feature.settings

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessDialog
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters

enum class DataPlace { Permanent, Erasable, PhoneOnly }

data class DataSection(val place: DataPlace, val title: String, val icon: WitnessIcon, val rows: List<Pair<WitnessIcon, String>>)

object PrivacyDataCopy {
    val SECTIONS = listOf(
        DataSection(
            DataPlace.Permanent,
            "On the Internet Computer · permanent",
            WitnessIcon.Lock,
            listOf(WitnessIcon.Tag to "Hashes and commitments", WitnessIcon.Key to "Key IDs · rotated, unlinkable", WitnessIcon.Schedule to "Capture and anchor times"),
        ),
        DataSection(
            DataPlace.Erasable,
            "On the Internet Computer · erasable",
            WitnessIcon.Delete,
            listOf(WitnessIcon.Fingerprint to "Fingerprints and region grids", WitnessIcon.Image to "Previews and proof pages", WitnessIcon.Link to "Encrypted case links"),
        ),
        DataSection(
            DataPlace.PhoneOnly,
            "Only on this phone",
            WitnessIcon.Smartphone,
            listOf(WitnessIcon.PhotoLibrary to "Original photos", WitnessIcon.LocationOn to "Exact locations", WitnessIcon.VpnKey to "Signing keys · hardware-backed"),
        ),
    )
    const val ERASE_TITLE = "Erase all public copies?"
    const val ERASE_TEXT = "Fingerprints, previews, proof pages and case links are removed from the Internet Computer. Anonymous proofs stay, and your originals stay on this phone."
    const val ITEMS_TITLE = "Public items"
    const val ERASE_ITEM = "Erase public copies"
    const val ERASE_ITEM_TITLE = "Erase this item's public copies?"
    const val ERASE_ITEM_TEXT = "Its fingerprints, preview, proof page and posts are removed from the Internet Computer. Its proof stays, and the original stays on this phone."

    fun eraseItemText(item: PublicItemUi): String = "${item.title} · ${item.subtitle}.\n\n$ERASE_ITEM_TEXT"
}

@Composable
fun PrivacyDataRoute(viewModel: PrivacyDataViewModel, navigator: SettingsNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    PrivacyDataScreen(
        state,
        onBack = navigator::back,
        onExport = viewModel::export,
        onErase = viewModel::requestErase,
        onCancelErase = viewModel::cancelErase,
        onConfirmErase = viewModel::confirmErase,
        onDismissNotice = viewModel::dismissNotice,
        onEraseItem = viewModel::requestEraseItem,
        onConfirmEraseItem = viewModel::confirmEraseItem,
    )
}

@Composable
fun PrivacyDataScreen(
    state: PrivacyDataUiState,
    onBack: () -> Unit = {},
    onExport: () -> Unit = {},
    onErase: () -> Unit = {},
    onCancelErase: () -> Unit = {},
    onConfirmErase: () -> Unit = {},
    onDismissNotice: () -> Unit = {},
    modifier: Modifier = Modifier,
    onEraseItem: (RecordId) -> Unit = {},
    onConfirmEraseItem: () -> Unit = {},
) {
    val palette = Witness.colors
    SettingsPage(
        "Privacy & data",
        onBack,
        modifier,
        overlay = {
            if (state.confirmErase) {
                WitnessDialog(
                    PrivacyDataCopy.ERASE_TITLE,
                    onCancelErase,
                    "Erase",
                    onConfirmErase,
                    text = PrivacyDataCopy.ERASE_TEXT,
                    icon = WitnessIcon.Delete,
                    destructive = true,
                )
            }
            state.confirmItem?.let { item ->
                WitnessDialog(
                    PrivacyDataCopy.ERASE_ITEM_TITLE,
                    onCancelErase,
                    "Erase",
                    onConfirmEraseItem,
                    text = PrivacyDataCopy.eraseItemText(item),
                    icon = WitnessIcon.Delete,
                    destructive = true,
                )
            }
        },
    ) {
        for (section in PrivacyDataCopy.SECTIONS) {
            val headColor = if (section.place == DataPlace.Permanent) palette.ink else palette.ink2
            Row(Modifier.padding(top = 4.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                WIcon(section.icon, headColor, size = 16.dp)
                Spacer(Modifier.width(6.dp))
                Overline(section.title, color = headColor)
            }
            ListCard {
                section.rows.forEachIndexed { i, (icon, text) ->
                    if (i > 0) RowDivider(start = 52.dp)
                    ListRow(icon, text, titleStyle = Witness.type.body, minHeight = 44.dp)
                }
            }
            Spacer(Modifier.height(20.dp))
        }
        Spacer(Modifier.height(20.dp))
        state.notice?.let {
            NoticeBanner(it, onDismissNotice)
            Spacer(Modifier.height(12.dp))
        }
        ListCard {
            ActionItem(
                WitnessIcon.Download,
                if (state.busy == PrivacyBusy.Exporting) "Exporting…" else "Export my data",
                "Proof records, case files and a JSON summary",
                palette.primary,
                enabled = state.busy == null,
                onClick = onExport,
            )
            RowDivider()
            ActionItem(
                WitnessIcon.Delete,
                if (state.busy == PrivacyBusy.Erasing) "Erasing…" else "Erase all public copies",
                "Fingerprints, previews, pages and links. Proofs stay.",
                palette.crit,
                enabled = state.busy == null,
                onClick = onErase,
            )
        }
        if (state.items.isNotEmpty()) {
            Spacer(Modifier.height(20.dp))
            Row(Modifier.padding(bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                WIcon(WitnessIcon.Image, palette.ink2, size = 16.dp)
                Spacer(Modifier.width(6.dp))
                Overline(PrivacyDataCopy.ITEMS_TITLE, color = palette.ink2)
            }
            ListCard {
                state.items.forEachIndexed { i, item ->
                    if (i > 0) RowDivider(start = 52.dp)
                    val erasing = state.erasingItem == item.recordId
                    ActionItem(
                        WitnessIcon.Delete,
                        if (erasing) "Erasing…" else PrivacyDataCopy.ERASE_ITEM,
                        Formatters.joinDot(item.title, item.subtitle),
                        palette.crit,
                        enabled = state.busy == null,
                        onClick = { onEraseItem(item.recordId) },
                    )
                }
            }
        }
        Spacer(Modifier.height(20.dp))
        Text(
            "Erasing is signed by this phone's key, or from any device with a linked Internet Identity.",
            style = SettingsText.footnote,
            color = palette.muted,
        )
    }
}

@Composable
private fun ActionItem(icon: WitnessIcon, title: String, subtitle: String, color: Color, enabled: Boolean, onClick: () -> Unit) {
    ListRow(
        icon,
        title,
        subtitle = subtitle,
        tint = color,
        titleColor = color,
        titleStyle = Witness.type.bodyStrong,
        minHeight = 64.dp,
        onClick = if (enabled) onClick else null,
    )
}
