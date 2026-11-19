package app.witness.feature.cases

import app.witness.core.model.LocalCaseId

enum class FlagReason(val label: String, val hint: String?) {
    WRONG("Wrong", "Doesn't match the photos"),
    MISLEADING("Misleading", "Right facts, wrong impression"),
    OFFENSIVE("Offensive", null),
    OTHER("Other", null),
}

data class GeneratedOutput(val target: String, val label: String, val value: String)

data class AiFlag(val caseId: LocalCaseId, val target: String, val reason: FlagReason, val note: String?, val atMs: Long)

interface AiFeedback {
    suspend fun flag(flag: AiFlag)

    suspend fun remove(caseId: LocalCaseId, target: String)
}

class InMemoryAiFeedback : AiFeedback {
    private val flagged = ArrayList<AiFlag>()
    private val removed = ArrayList<Pair<LocalCaseId, String>>()

    val flags: List<AiFlag> get() = flagged.toList()
    val removals: List<Pair<LocalCaseId, String>> get() = removed.toList()

    override suspend fun flag(flag: AiFlag) {
        flagged += flag
    }

    override suspend fun remove(caseId: LocalCaseId, target: String) {
        removed += caseId to target
    }
}
