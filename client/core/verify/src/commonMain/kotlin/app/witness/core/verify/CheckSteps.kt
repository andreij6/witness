package app.witness.core.verify

class CheckSteps private constructor(private val states: Array<StageState>, val answered: Int, val total: Int, val judgement: Judgement?) {
    operator fun get(stage: CheckStage): StageState = states[stage.ordinal]

    val current: CheckStage? get() = CheckStage.entries.firstOrNull { states[it.ordinal] == StageState.RUNNING }

    val isDone: Boolean get() = judgement != null

    fun reduce(update: CheckUpdate): CheckSteps = when (update) {
        is CheckUpdate.Done -> CheckSteps(
            Array(states.size) { if (states[it] == StageState.RUNNING || states[it] == StageState.WAITING) StageState.SKIPPED else states[it] },
            answered,
            total,
            update.judgement,
        )
        is CheckUpdate.Progress -> {
            val next = states.copyOf()
            next[update.stage.ordinal] = update.state
            val isShards = update.stage == CheckStage.FINGERPRINTS && update.total > 0
            CheckSteps(next, if (isShards) update.answered else answered, if (isShards) update.total else total, judgement)
        }
    }

    override fun equals(other: Any?): Boolean =
        other is CheckSteps && states.contentEquals(other.states) && answered == other.answered && total == other.total && judgement == other.judgement

    override fun hashCode(): Int = (states.contentHashCode() * 31 + answered) * 31 + total

    override fun toString(): String = "CheckSteps(${CheckStage.entries.joinToString { "$it=${states[it.ordinal]}" }}, $answered/$total, $judgement)"

    companion object {
        val INITIAL: CheckSteps = CheckSteps(Array(CheckStage.entries.size) { StageState.WAITING }, 0, 0, null)
    }
}
