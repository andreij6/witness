package app.witness.core.domain.usecase

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.model.ByteString
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.EnumBits
import app.witness.core.model.Platform
import app.witness.core.model.StageState
import app.witness.core.model.StageStatus
import app.witness.core.model.VerdictReason
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

data class CheckProgress(val stages: List<StageStatus>, val result: CheckResult? = null, val error: Throwable? = null) {
    val isDone: Boolean get() = result != null || error != null

    operator fun get(stage: CheckStage): StageStatus = stages[stage.ordinal]

    companion object {
        val INITIAL: CheckProgress = CheckProgress(CheckStage.entries.map { StageStatus(it, StageState.WAITING) })
    }
}

class CheckImage(
    private val engine: VerifyEngine,
    private val registry: RegistryGateway,
    private val index: IndexGateway,
    private val media: MediaGateway,
    private val platform: Platform,
    private val telemetry: Telemetry = Telemetry.None,
) {
    operator fun invoke(input: ByteString, options: JudgeOptions = JudgeOptions.forPlatform(platform)): Flow<CheckProgress> = channelFlow {
        val lock = Mutex()
        var current = CheckProgress.INITIAL
        send(current)
        suspend fun emit(transform: (List<StageStatus>) -> List<StageStatus>) = lock.withLock {
            current = current.copy(stages = transform(current.stages))
            send(current)
        }
        if (input.size == 0) {
            emit { s -> s.withStage(CheckStage.READ_FILE, StageState.FAILED).settle() }
            send(current.copy(result = CheckResult.NO_RECORD.copy(reasons = EnumBits.of(VerdictReason.UNDECODABLE))))
            return@channelFlow
        }
        val sink = StageSink { stage, state, done, total, detail ->
            emit { s -> s.withStage(stage, state, done, total, detail) }
        }
        val oracle = GatewayOracle(registry, index, media, options, sink)
        val result = try {
            engine.judge(input, oracle, options, sink)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            emit { s -> s.fail() }
            send(current.copy(error = e))
            return@channelFlow
        }
        emit { s -> s.settle() }
        send(current.copy(result = result))
        telemetry.event(AnalyticsEvent.CHECK_COMPLETED, mapOf("verdict" to result.verdict.ordinal.toLong(), "platform" to platform.ordinal.toLong()))
    }

    companion object {
        internal fun List<StageStatus>.withStage(stage: CheckStage, state: StageState, done: Int = 0, total: Int = 0, detail: String? = null): List<StageStatus> {
            val prev = this[stage.ordinal]
            if (prev.state.isTerminal && !state.isTerminal) return this
            val next = StageStatus(stage, state, done, total, detail ?: prev.detail)
            return if (next == prev) this else toMutableList().also { it[stage.ordinal] = next }
        }

        internal fun List<StageStatus>.settle(): List<StageStatus> = map {
            when (it.state) {
                StageState.RUNNING -> it.copy(state = StageState.DONE)
                StageState.WAITING -> it.copy(state = StageState.SKIPPED)
                else -> it
            }
        }

        internal fun List<StageStatus>.fail(): List<StageStatus> = map {
            when (it.state) {
                StageState.RUNNING -> it.copy(state = StageState.FAILED)
                StageState.WAITING -> it.copy(state = StageState.SKIPPED)
                else -> it
            }
        }

        private val StageState.isTerminal: Boolean
            get() = this == StageState.DONE || this == StageState.SKIPPED || this == StageState.FAILED
    }
}
