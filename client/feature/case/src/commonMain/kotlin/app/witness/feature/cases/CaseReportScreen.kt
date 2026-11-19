package app.witness.feature.cases

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.AiHeader
import app.witness.core.designsystem.BottomBar
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.FlagButton
import app.witness.core.designsystem.GeneratedSurface
import app.witness.core.designsystem.GeneratedTag
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.SealBadge
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessDialog
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.dashedBorder
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId

data class CaseReportHost(
    val caseId: LocalCaseId,
    val templateId: TemplateId,
    val onFlag: (GeneratedOutput) -> Unit,
    val onCaptureFor: (String) -> Unit,
    val summary: String? = null,
    val onEdit: (String) -> Unit = {},
    val summarySource: SummarySource? = null,
)

typealias CaseReportSlot = @Composable (CaseReportHost) -> Unit

class CaseReportActions(
    val onBack: () -> Unit,
    val onOpenItem: (RecordId) -> Unit,
    val onCapture: (target: String?) -> Unit,
    val onExport: () -> Unit,
    val onShare: () -> Unit,
    val onChangeType: () -> Unit,
    val onChooseType: (ReportType) -> Unit,
    val onFlag: (GeneratedOutput) -> Unit,
    val onFlagReason: (FlagReason) -> Unit,
    val onFlagNote: (String) -> Unit,
    val onSendFlag: () -> Unit,
    val onRemoveFlagged: () -> Unit,
    val onEditSummary: () -> Unit,
    val onSummaryDraft: (String) -> Unit,
    val onSaveSummary: () -> Unit,
    val onRequestDelete: () -> Unit,
    val onConfirmDelete: () -> Unit,
    val onDismissSheet: () -> Unit,
    val onClearError: () -> Unit,
)

fun editor(actions: CaseReportActions): (String) -> Unit = { target -> if (target == CaseReportViewModel.SUMMARY_TARGET) actions.onEditSummary() }

fun CaseReportViewModel.actions(onBack: () -> Unit, onOpenItem: (RecordId) -> Unit, onCapture: (String?) -> Unit, onExport: () -> Unit, onShare: () -> Unit) =
    CaseReportActions(
        onBack, onOpenItem, onCapture, onExport, onShare,
        ::openReportTypes, ::chooseReportType, ::openFlag, ::setFlagReason, ::setFlagNote, ::sendFlag, ::removeFlagged,
        ::editSummary, ::setSummaryDraft, ::saveSummary, ::requestDelete, ::confirmDelete, ::dismissSheet, ::clearError,
    )

@Composable
fun CaseReportScreen(
    state: CaseReportUiState,
    actions: CaseReportActions,
    modifier: Modifier = Modifier,
    image: RecordImage = {},
    report: CaseReportSlot? = null,
) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars)) {
            WitnessAppBar(state.title, subtitle = state.subtitle, onNavigate = actions.onBack, actions = { MoreMenu(actions.onRequestDelete) })
            LazyColumn(Modifier.weight(1f), contentPadding = PaddingValues(top = 4.dp, bottom = 104.dp)) {
                item(key = "banner") { ReportBanner(state.reportLabel, actions.onChangeType) }
                item(key = "evidence-head") { EvidenceHeader(state.evidenceTitle, state.anchoringLine) }
                item(key = "evidence") { EvidenceStrip(state.items, image, actions.onOpenItem) { actions.onCapture(null) } }
                item(key = "report") {
                    Column(Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp)) {
                        if (report != null) {
                            report(CaseReportHost(state.caseId, state.templateId, actions.onFlag, actions.onCapture, state.summary, editor(actions), state.summarySource))
                        } else {
                            SummaryCard(state.summary, state.summarySource, actions.onEditSummary) { text ->
                                actions.onFlag(GeneratedOutput(CaseReportViewModel.SUMMARY_TARGET, SUMMARY_LABEL, text))
                            }
                        }
                        ErrorLine(state.error, Modifier.padding(top = 12.dp), actions.onClearError)
                    }
                }
            }
        }
        BottomBar {
            WitnessButton("Export ZIP", actions.onExport, Modifier.width(128.dp), ButtonKind.Tonal)
            WitnessButton("Share case", actions.onShare, Modifier.weight(1f), icon = WitnessIcon.Share)
        }
        CaseSheets(state.sheet, actions)
    }
}

internal const val SUMMARY_LABEL = SummaryCopy.DRAFT

object SummaryCopy {
    const val DRAFT = "Draft summary"
    const val EDITED = "Summary · edited by you"
    const val WRITTEN = "Summary · written by you"
    const val EMPTY = "Summary"

    fun label(source: SummarySource?): String = when (source) {
        SummarySource.GENERATED -> DRAFT
        SummarySource.EDITED -> EDITED
        SummarySource.WRITTEN -> WRITTEN
        null -> EMPTY
    }

    fun isGenerated(source: SummarySource?): Boolean = source == SummarySource.GENERATED
}

@Composable
private fun MoreMenu(onDelete: () -> Unit) {
    var open by remember { mutableStateOf(false) }
    Box {
        WitnessIconButton(WitnessIcon.MoreVert, "More options", { open = true })
        DropdownMenu(open, { open = false }) {
            DropdownMenuItem(
                text = { Text("Delete case", style = Witness.type.body, color = Witness.colors.crit) },
                onClick = {
                    open = false
                    onDelete()
                },
            )
        }
    }
}

@Composable
private fun ReportBanner(label: String, onChange: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier
            .padding(horizontal = 16.dp)
            .fillMaxWidth()
            .height(40.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(palette.ai.copy(alpha = 0.08f))
            .padding(start = 12.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.AutoAwesome, palette.ai, size = 20.dp)
        Spacer(Modifier.width(8.dp))
        Text(label, Modifier.weight(1f), style = Witness.type.bodyCompact.merge(SemiBoldWeight), color = palette.ai, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text(
            "Change",
            Modifier.clip(RoundedCornerShape(8.dp)).clickable(role = Role.Button, onClick = onChange).padding(horizontal = 8.dp, vertical = 6.dp),
            style = Witness.type.bodyCompact.merge(SemiBoldWeight),
            color = palette.ai,
        )
    }
}

@Composable
private fun EvidenceHeader(title: String, line: String) {
    Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 9.dp), verticalAlignment = Alignment.CenterVertically) {
        Overline(title, Modifier.weight(1f))
        Text(line, style = Witness.type.caption.merge(MediumWeight), color = Witness.colors.muted, maxLines = 1)
    }
}

internal val ThumbWidth = 84.dp
internal val ThumbHeight = 108.dp
internal val ThumbShape = RoundedCornerShape(12.dp)

@Composable
private fun EvidenceStrip(items: List<EvidenceItemUi>, image: RecordImage, onOpen: (RecordId) -> Unit, onAdd: () -> Unit) {
    val palette = Witness.colors
    Box {
        LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(items, key = { it.recordId.text }) { item ->
                Box(Modifier.size(ThumbWidth, ThumbHeight).clip(ThumbShape).background(palette.surface2).clickable(role = Role.Button) { onOpen(item.recordId) }) {
                    image(item.recordId)
                    SealBadge(item.state, Modifier.align(Alignment.BottomEnd).padding(1.dp), radius = 9.dp, ring = palette.paper)
                }
            }
            item(key = "add") {
                Box(
                    Modifier
                        .size(ThumbWidth, ThumbHeight)
                        .clip(ThumbShape)
                        .background(palette.ai.copy(alpha = 0.06f))
                        .dashedBorder(palette.ai.copy(alpha = 0.6f), 12.dp, dash = 5.dp, gap = 4.dp)
                        .clickable(role = Role.Button, onClick = onAdd)
                        .semantics { contentDescription = "Capture more evidence" },
                    contentAlignment = Alignment.Center,
                ) { WIcon(WitnessIcon.Add, palette.ai, size = 22.dp) }
            }
        }
        Box(
            Modifier
                .align(Alignment.CenterEnd)
                .size(44.dp, ThumbHeight + 4.dp)
                .background(Brush.horizontalGradient(listOf(Color.Transparent, palette.paper))),
        )
    }
}

@Composable
private fun SummaryCard(summary: String?, source: SummarySource?, onEdit: () -> Unit, onFlag: (String) -> Unit) {
    val palette = Witness.colors
    val edit: @Composable () -> Unit = {
        Text(
            "Edit",
            Modifier.clip(RoundedCornerShape(8.dp)).clickable(role = Role.Button, onClick = onEdit).padding(horizontal = 6.dp, vertical = 4.dp),
            style = Witness.type.labelStrong,
            color = palette.primary,
        )
    }
    val body: @Composable () -> Unit = {
        Spacer(Modifier.height(6.dp))
        Text(summary ?: SUMMARY_PLACEHOLDER, style = Witness.type.subtitle, color = if (summary != null) palette.ink2 else palette.muted)
    }
    if (summary != null && SummaryCopy.isGenerated(source)) {
        GeneratedSurface(Modifier.fillMaxWidth()) {
            AiHeader(SummaryCopy.label(source), trailing = {
                FlagButton({ onFlag(summary) })
                edit()
            })
            body()
        }
    } else {
        WitnessCard(Modifier.fillMaxWidth()) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Overline(SummaryCopy.label(source.takeIf { summary != null }), Modifier.weight(1f))
                edit()
            }
            body()
        }
    }
}

internal const val SUMMARY_PLACEHOLDER = "Add a short, neutral summary of what happened."

@Composable
private fun CaseSheets(sheet: CaseSheet?, actions: CaseReportActions) {
    val types = rememberLast(sheet as? CaseSheet.ReportTypes)
    WitnessSheet(sheet is CaseSheet.ReportTypes, actions.onDismissSheet) { types?.let { ReportTypeSheet(it, actions.onChooseType) } }
    val flag = rememberLast(sheet as? CaseSheet.Flag)
    WitnessSheet(sheet is CaseSheet.Flag, actions.onDismissSheet) { flag?.let { FlagSheet(it, actions) } }
    val summary = rememberLast(sheet as? CaseSheet.Summary)
    WitnessSheet(sheet is CaseSheet.Summary, actions.onDismissSheet) { summary?.let { SummarySheet(it, actions) } }
    if (sheet is CaseSheet.ConfirmDelete) {
        WitnessDialog(
            title = "Delete this case?",
            text = "Your photos stay in Home. Shared links keep working until they expire.",
            onDismiss = actions.onDismissSheet,
            confirmLabel = "Delete",
            onConfirm = actions.onConfirmDelete,
            icon = WitnessIcon.Delete,
            destructive = true,
        )
    }
}

@Composable
internal fun ReportTypeSheet(sheet: CaseSheet.ReportTypes, onChoose: (ReportType) -> Unit) {
    SheetTitle("Change report type", bottom = 12.dp)
    if (sheet.suggested != null) GeneratedTag("AI suggested: " + ReportType.label(sheet.suggested), Modifier.padding(start = 24.dp))
    Spacer(Modifier.height(16.dp))
    sheet.options.forEach { type ->
        OptionRow(
            icon = type.rowIcon,
            title = type.title,
            subtitle = type.subtitle,
            selected = type.available && type.templateId == sheet.selected,
            onClick = if (type.available) ({ onChoose(type) }) else null,
            muted = !type.available,
            trailing = { if (type.available) Radio(type.templateId == sheet.selected) else LaterChip() },
        )
        Spacer(Modifier.height(4.dp))
    }
    InfoNote(
        "Changing type keeps your photos and moves matching details.",
        Modifier.padding(start = 24.dp, end = 24.dp, top = 14.dp, bottom = 24.dp),
        style = Witness.type.subtitle,
    )
}

@Composable
internal fun FlagSheet(sheet: CaseSheet.Flag, actions: CaseReportActions) {
    val palette = Witness.colors
    SheetTitle("What's wrong with this?")
    GeneratedSurface(Modifier.padding(horizontal = 16.dp).fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    WIcon(WitnessIcon.AutoAwesome, palette.ai, size = 16.dp)
                    Spacer(Modifier.width(6.dp))
                    Overline(sheet.output.label, color = palette.ai)
                }
                Spacer(Modifier.height(6.dp))
                Text(sheet.output.value, style = Witness.type.cardTitle, color = palette.ink, maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
            WIcon(WitnessIcon.Flag, palette.ink, size = 20.dp)
        }
    }
    Overline("Reason", Modifier.padding(start = 24.dp, top = 20.dp, bottom = 6.dp))
    FlagReason.entries.forEach { reason -> ReasonRow(reason, reason == sheet.reason) { actions.onFlagReason(reason) } }
    NoteField(sheet.note, actions.onFlagNote, "Tell us more (optional)", Modifier.padding(start = 16.dp, end = 16.dp, top = 8.dp))
    InfoNote("Flags stay on your phone unless you export feedback.", Modifier.padding(start = 24.dp, end = 24.dp, top = 16.dp), WitnessIcon.Lock)
    Row(Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 24.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        OutlinedTone("Remove it", WitnessIcon.Delete, palette.crit, actions.onRemoveFlagged, Modifier.weight(1f), enabled = !sheet.busy)
        WitnessButton("Send flag", actions.onSendFlag, Modifier.weight(1f), icon = WitnessIcon.Send, enabled = !sheet.busy)
    }
}

@Composable
private fun ReasonRow(reason: FlagReason, selected: Boolean, onSelect: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier
            .fillMaxWidth()
            .height(44.dp)
            .clickable(role = Role.RadioButton, onClick = onSelect)
            .padding(start = 20.dp, end = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Radio(selected)
        Spacer(Modifier.width(12.dp))
        Text(reason.label, style = Witness.type.actionTitle.merge(if (selected) SemiBoldWeight else MediumWeight), color = palette.ink)
        if (reason.hint != null) {
            Spacer(Modifier.width(8.dp))
            Text(reason.hint, style = Witness.type.subtitle, color = palette.muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun SummarySheet(sheet: CaseSheet.Summary, actions: CaseReportActions) {
    SheetTitle(SUMMARY_LABEL)
    NoteField(sheet.text, actions.onSummaryDraft, SUMMARY_PLACEHOLDER, Modifier.padding(horizontal = 16.dp), minHeight = 140.dp)
    WitnessButton("Save", actions.onSaveSummary, Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 24.dp).fillMaxWidth())
}
