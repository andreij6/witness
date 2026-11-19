package app.witness.core.verify

import app.witness.core.icp.id.RecordId
import app.witness.core.model.Verdict

data class Judgement(
    val verdict: Verdict,
    val recordId: RecordId? = null,
    val flags: Set<VerdictFlag> = emptySet(),
    val fallback: Boolean = false,
) {
    companion object {
        val NO_RECORD: Judgement = Judgement(Verdict.NO_RECORD)
    }
}
