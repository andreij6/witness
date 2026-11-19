package app.witness.core.domain.fake

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.domain.port.update
import app.witness.core.model.Anchor
import app.witness.core.model.ByteString
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.EnumBits
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Reference
import app.witness.core.model.StageState
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason

class FakeVerifyEngine(var failure: Exception? = null) : VerifyEngine {
    val seen = ArrayList<JudgeOptions>()

    override suspend fun judge(input: ByteString, oracle: VerifyOracle, options: JudgeOptions, stages: StageSink): CheckResult {
        seen += options
        stages.update(CheckStage.READ_FILE, StageState.RUNNING)
        failure?.let { throw it }
        stages.update(CheckStage.READ_FILE, StageState.DONE, "JPEG")
        stages.update(CheckStage.CREDENTIALS, StageState.DONE)
        val digest = ByteString.of(SoftwareSha256.digest(input.toByteArray()))
        oracle.exact(digest, null).firstOrNull()?.let { r ->
            return CheckResult(
                Verdict.ORIGINAL,
                reasons = EnumBits.of(VerdictReason.EXACT_ASSET_HASH),
                reference = Reference(r.recordId, r.claim, Anchor(r.recordIx, r.anchoredAtMs), MatchEvidence.EXACT_HASH),
            )
        }
        val probe = oracle.pdq(listOf(digest))
        stages.update(CheckStage.FINGERPRINTS, StageState.DONE, probe.shardsAnswered, probe.shardsAsked, null)
        val confirmed = probe.hits.take(options.topK)
        if (confirmed.isEmpty()) {
            return CheckResult(Verdict.NO_RECORD, reasons = EnumBits.of(VerdictReason.NO_CANDIDATES), flags = truncation(probe.truncated))
        }
        stages.update(CheckStage.COMPARE, StageState.RUNNING)
        val best = confirmed.first().record
        val reference = oracle.reference(best)
        val flags = truncation(probe.truncated).with(VerdictFlag.NO_PUBLIC_REFERENCE, reference == null)
        return CheckResult(
            Verdict.MATCHES_WITH_CHANGES,
            flags = flags,
            reference = Reference(best.recordId, best.claim, Anchor(best.recordIx, best.anchoredAtMs), if (reference != null) MatchEvidence.PIXELS else MatchEvidence.GRID),
            declared = best.claim.kind.edits,
        )
    }

    private fun truncation(truncated: Boolean) = EnumBits.none<VerdictFlag>().with(VerdictFlag.TRUNCATED_CANDIDATES, truncated)
}
