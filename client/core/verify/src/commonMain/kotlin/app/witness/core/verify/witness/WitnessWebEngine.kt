package app.witness.core.verify.witness

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.icp.witness.WitnessBridge
import app.witness.core.model.ByteString
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.StageState
import kotlinx.coroutines.async
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope

class StageUpdate(val stage: CheckStage, val state: StageState, val completed: Int, val total: Int) {
    companion object {
        fun decode(stage: Int, state: Int, completed: Int, total: Int): StageUpdate? {
            val s = CheckStage.entries.getOrNull(stage) ?: return null
            val st = StageState.entries.getOrNull(state) ?: return null
            if (completed < 0 || total < 0 || completed > total) return null
            return StageUpdate(s, st, completed, total)
        }
    }
}

class WitnessWebEngine(private val bridge: WitnessBridge) : VerifyEngine {
    override suspend fun judge(input: ByteString, oracle: VerifyOracle, options: JudgeOptions, stages: StageSink): CheckResult = coroutineScope {
        val updates = Channel<StageUpdate>(Channel.UNLIMITED)
        val verdict = async {
            try {
                bridge.check(input.toByteArray()) { s, st, done, total -> StageUpdate.decode(s, st, done, total)?.let(updates::trySend) }
            } finally {
                updates.close()
            }
        }
        for (u in updates) stages.update(u.stage, u.state, u.completed, u.total, null)
        VerdictJson.parse(verdict.await())
    }
}
