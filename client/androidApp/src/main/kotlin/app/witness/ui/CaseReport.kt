package app.witness.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import app.witness.core.a2ui.A2uiMessages
import app.witness.core.a2ui.Template
import app.witness.core.a2ui.WitnessEvent
import app.witness.core.a2ui.WitnessEventSink
import app.witness.core.a2ui.WitnessReport
import app.witness.core.a2ui.WitnessReportSurface
import app.witness.core.a2ui.ui.LocalSummaryHeader
import app.witness.core.a2ui.ui.SummaryHeader
import app.witness.di.BundledTemplates
import app.witness.feature.cases.CaseReportHost
import app.witness.feature.cases.CaseReportSlot
import app.witness.feature.cases.CaseReportViewModel
import app.witness.feature.cases.GeneratedOutput
import app.witness.feature.cases.SummaryCopy
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

object CaseReport {
    fun route(event: WitnessEvent, host: CaseReportHost) = when (event) {
        is WitnessEvent.CaptureFor -> host.onCaptureFor(event.target)
        is WitnessEvent.Flag -> host.onFlag(GeneratedOutput(event.target, event.target, ""))
        is WitnessEvent.Edit -> host.onEdit(event.target)
        is WitnessEvent.Remove -> Unit
    }

    fun slot(templates: BundledTemplates): CaseReportSlot = { host -> A2uiCaseReport(host, templates) }

    fun summaryHeader(host: CaseReportHost): SummaryHeader {
        val source = host.summarySource.takeIf { host.summary != null }
        return SummaryHeader(if (SummaryCopy.isGenerated(source)) null else SummaryCopy.label(source), SummaryCopy.isGenerated(source))
    }

    fun summaryUpdate(template: Template, summary: String?): JsonObject =
        A2uiMessages.updateDataModel(template.surfaceId, A2uiMessages.slotPath(CaseReportViewModel.SUMMARY_TARGET), summary?.let(::JsonPrimitive) ?: JsonNull)
}

@Composable
private fun A2uiCaseReport(host: CaseReportHost, templates: BundledTemplates) {
    val current by rememberUpdatedState(host)
    val template by produceState<Template?>(null, host.templateId) {
        value = runCatching { templates.store()[host.templateId.value] }.getOrNull()
    }
    val shown = template ?: return
    val report = remember(host.caseId) { WitnessReport(WitnessEventSink { CaseReport.route(it, current) }) }
    LaunchedEffect(report) { report.run() }
    LaunchedEffect(report, shown) { report.show(shown) }
    LaunchedEffect(report, shown, host.summary) { report.send(CaseReport.summaryUpdate(shown, host.summary)) }
    CompositionLocalProvider(LocalSummaryHeader provides CaseReport.summaryHeader(host)) { WitnessReportSurface(report, shown.surfaceId) }
}
