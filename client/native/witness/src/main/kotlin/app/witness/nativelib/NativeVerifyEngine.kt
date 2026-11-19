package app.witness.nativelib

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.domain.port.update
import app.witness.core.model.ByteString
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.StageState
import app.witness.core.model.VerdictReason
import uniffi.witness_android.WitnessInterface

class NativeVerifyEngine(private val witness: WitnessInterface) : VerifyEngine {
    override suspend fun judge(input: ByteString, oracle: VerifyOracle, options: JudgeOptions, stages: StageSink): CheckResult {
        stages.update(CheckStage.READ_FILE, StageState.RUNNING)
        val result = NativeMapping.verdict(remote(NativeTimeouts.LONG_MS) { witness.check(input.toByteArray(), null) })
        for ((stage, state) in stagesOf(result)) stages.update(stage, state)
        return result
    }

    companion object {
        fun stagesOf(result: CheckResult): List<Pair<CheckStage, StageState>> {
            val undecodable = VerdictReason.UNDECODABLE in result.reasons
            fun after(state: StageState) = if (undecodable) StageState.SKIPPED else state
            return listOf(
                CheckStage.READ_FILE to if (undecodable) StageState.FAILED else StageState.DONE,
                CheckStage.CREDENTIALS to after(StageState.DONE),
                CheckStage.FINGERPRINTS to after(StageState.DONE),
                CheckStage.WATERMARK to after(if (VerdictReason.WATERMARK_UNAVAILABLE in result.reasons) StageState.SKIPPED else StageState.DONE),
                CheckStage.COMPARE to after(if (result.reference != null) StageState.DONE else StageState.SKIPPED),
            )
        }
    }
}
