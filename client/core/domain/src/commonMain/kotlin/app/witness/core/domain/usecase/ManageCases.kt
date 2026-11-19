package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureMode
import app.witness.core.model.Case
import app.witness.core.model.CaseSummary
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first

class CaseException(message: String) : Exception(message)

class ManageCases(
    private val cases: CaseStore,
    private val captures: CaptureStore,
    private val settings: SettingsStore,
    private val catalog: TemplateCatalog,
    private val assistant: Assistant,
    private val clock: Clock,
    private val telemetry: Telemetry = Telemetry.None,
) {
    fun observe(): Flow<List<CaseSummary>> = combine(cases.observeAll(), captures.observeAll(), ::summarize)

    suspend fun create(title: String, templateId: TemplateId? = null, activate: Boolean = true): Case {
        val clean = cleanTitle(title)
        val template = templateId ?: suggestTemplate(clean)
        val case = cases.create(clean, template, clock.nowEpochMillis())
        if (activate) setActive(case.id)
        telemetry.event(AnalyticsEvent.CASE_CREATED, mapOf("template_known" to if (template == TemplateId.GENERAL) 0L else 1L))
        return case
    }

    suspend fun rename(id: LocalCaseId, title: String): Case = edit(id) { it.copy(title = cleanTitle(title)) }

    suspend fun changeTemplate(id: LocalCaseId, templateId: TemplateId): Case = edit(id) { it.copy(templateId = templateId) }

    suspend fun updateSummary(id: LocalCaseId, summary: String?): Case = edit(id) {
        val clean = PostText.clean(summary, MAX_SUMMARY)
        it.copy(summary = clean, summarySource = SummarySource.afterUserEdit(it.summarySource, it.summary, clean))
    }

    suspend fun draftSummary(id: LocalCaseId, summary: String?): Case = edit(id) {
        val clean = PostText.clean(summary, MAX_SUMMARY)
        it.copy(summary = clean, summarySource = clean?.let { SummarySource.GENERATED })
    }

    suspend fun setActive(id: LocalCaseId?) {
        if (id != null && cases.get(id) == null) throw CaseException("unknown case")
        settings.update { it.copy(activeCase = id, captureMode = if (id != null) CaptureMode.CASE else CaptureMode.POST) }
    }

    suspend fun move(recordId: RecordId, to: LocalCaseId?) {
        val capture = captures.get(recordId) ?: throw CaseException("unknown capture")
        if (to != null && cases.get(to) == null) throw CaseException("unknown case")
        if (capture.caseId != to) captures.update(capture.copy(caseId = to))
    }

    suspend fun delete(id: LocalCaseId) {
        val members = captures.observeAll().first().filter { it.caseId == id }
        for (c in members) captures.update(c.copy(caseId = null))
        cases.delete(id)
        if (settings.settings.first().activeCase == id) setActive(null)
    }

    suspend fun suggestTemplate(title: String): TemplateId {
        val templates = catalog.templates()
        val rules = TemplateMatcher.pick(title, templates)
        if (!settings.settings.first().aiAssistant) return rules
        val ai = try {
            if (assistant.isAvailable()) assistant.suggest(AssistantContext(CaptureMode.CASE, title = title), templates)?.templateId else null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        }
        return ai?.takeIf { id -> templates.any { it.id == id } } ?: rules
    }

    private suspend fun edit(id: LocalCaseId, transform: (Case) -> Case): Case {
        val current = cases.get(id) ?: throw CaseException("unknown case")
        val next = transform(current).copy(updatedAtMs = clock.nowEpochMillis())
        cases.update(next)
        return next
    }

    companion object {
        const val MAX_TITLE: Int = 80
        const val MAX_SUMMARY: Int = 4_000
        const val COVER_COUNT: Int = 3

        fun cleanTitle(title: String): String = PostText.clean(title, MAX_TITLE) ?: throw CaseException("title required")

        fun summarize(cases: List<Case>, captures: List<Capture>): List<CaseSummary> {
            val byCase = HashMap<LocalCaseId, MutableList<Capture>>()
            for (c in captures) c.caseId?.let { byCase.getOrPut(it) { ArrayList() }.add(c) }
            return cases.sortedByDescending { it.updatedAtMs }.map { case ->
                val items = byCase[case.id].orEmpty()
                CaseSummary(
                    case = case,
                    itemCount = items.size,
                    anchoredCount = items.count { it.status == AnchoringStatus.ANCHORED },
                    queuedCount = items.count { it.status == AnchoringStatus.QUEUED || it.status == AnchoringStatus.SEALED },
                    coverRecords = items.sortedByDescending { it.createdAtMs }.take(COVER_COUNT).map { it.recordId },
                )
            }
        }
    }
}
