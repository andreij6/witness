package app.witness.core.model

enum class ShareExpiry(val durationMs: Long?) {
    DAY(24L * 3_600_000),
    WEEK(7L * 86_400_000),
    MONTH(30L * 86_400_000),
    NEVER(null);

    fun expiresAt(nowMs: Long): Long? = durationMs?.let { nowMs + it }
}

data class CaseShare(
    val caseId: CaseId,
    val url: String,
    val createdAtMs: Long,
    val expiresAtMs: Long?,
    val itemCount: Int,
) {
    fun isExpired(nowMs: Long): Boolean = expiresAtMs != null && nowMs >= expiresAtMs
}

data class Case(
    val id: LocalCaseId,
    val title: String,
    val templateId: TemplateId,
    val createdAtMs: Long,
    val updatedAtMs: Long = createdAtMs,
    val summary: String? = null,
    val shares: List<CaseShare> = emptyList(),
    val summarySource: SummarySource? = null,
)

enum class SummarySource {
    GENERATED,
    EDITED,
    WRITTEN;

    companion object {
        fun afterUserEdit(previous: SummarySource?, before: String?, after: String?): SummarySource? = when {
            after == null -> null
            before == null -> WRITTEN
            after == before -> previous ?: WRITTEN
            previous == GENERATED || previous == EDITED -> EDITED
            else -> WRITTEN
        }

        fun parse(name: String?): SummarySource? = entries.firstOrNull { it.name == name }
    }
}

data class CaseSummary(
    val case: Case,
    val itemCount: Int,
    val anchoredCount: Int,
    val queuedCount: Int,
    val coverRecords: List<RecordId>,
)
