package app.witness.core.verify

import app.witness.core.icp.id.RecordId
import app.witness.core.model.Verdict

object VerifyWire {
    fun verdict(code: Int): Verdict? = when (code) {
        0 -> Verdict.ORIGINAL
        1 -> Verdict.MATCHES_WITH_CHANGES
        2 -> Verdict.ALTERED
        3 -> Verdict.NO_RECORD
        4 -> Verdict.SEALED_UNANCHORED
        else -> null
    }

    fun code(verdict: Verdict): Int = when (verdict) {
        Verdict.ORIGINAL -> 0
        Verdict.MATCHES_WITH_CHANGES -> 1
        Verdict.ALTERED -> 2
        Verdict.NO_RECORD -> 3
        Verdict.SEALED_UNANCHORED -> 4
    }

    fun stage(code: Int): CheckStage? = CheckStage.entries.getOrNull(code)

    fun state(code: Int): StageState? = StageState.entries.getOrNull(code)

    fun progress(stage: Int, state: Int, answered: Int, total: Int): CheckUpdate.Progress? {
        val s = stage(stage) ?: return null
        val st = state(state) ?: return null
        if (answered < 0 || total < 0 || answered > total) return null
        return CheckUpdate.Progress(s, st, answered, total)
    }

    fun judgement(verdict: Int, recordId: String?, flags: Int, fallback: Boolean): Judgement {
        val v = verdict(verdict) ?: return Judgement.NO_RECORD.copy(fallback = fallback)
        val id = recordId?.let(RecordId::parse)
        if (v != Verdict.NO_RECORD && v != Verdict.ALTERED && v != Verdict.SEALED_UNANCHORED && id == null) {
            return Judgement.NO_RECORD.copy(fallback = fallback)
        }
        return Judgement(v, id, VerdictFlag.decode(flags), fallback)
    }
}
