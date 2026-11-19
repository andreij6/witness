package app.witness.core.domain.port

import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.Platform
import app.witness.core.model.RecordId
import app.witness.core.model.StageState

data class JudgeOptions(
    val platform: Platform,
    val topK: Int,
    val maxQueryHashes: Int = MAX_QUERY_HASHES,
    val maxExact: Int = MAX_EXACT,
    val maxWatermark: Int = MAX_WATERMARK,
    val maxDerivatives: Int = MAX_DERIVATIVES,
    val maxPdqCandidates: Int = MAX_PDQ_CANDIDATES,
    val maxWatermarkInferences: Int = MAX_WATERMARK_INFERENCES,
) {
    companion object {
        const val MAX_TOP_K: Int = 16
        const val MAX_QUERY_HASHES: Int = 16
        const val MAX_EXACT: Int = 8
        const val MAX_WATERMARK: Int = 32
        const val MAX_DERIVATIVES: Int = 32
        const val MAX_PDQ_CANDIDATES: Int = 32
        const val MAX_WATERMARK_INFERENCES: Int = 4

        fun forPlatform(platform: Platform, topK: Int = platform.topK): JudgeOptions =
            JudgeOptions(platform, topK.coerceIn(1, MAX_TOP_K))
    }
}

fun interface StageSink {
    suspend fun update(stage: CheckStage, state: StageState, completed: Int, total: Int, detail: String?)
}

suspend fun StageSink.update(stage: CheckStage, state: StageState, detail: String? = null) = update(stage, state, 0, 0, detail)

data class PdqHit(val record: AnchoredRecord, val distance: Int)

data class PdqProbe(val hits: List<PdqHit>, val truncated: Boolean, val shardsAsked: Int, val shardsAnswered: Int)

interface VerifyOracle {
    suspend fun key(keyId: KeyId): KeyInfo?

    suspend fun record(recordId: RecordId): AnchoredRecord?

    suspend fun exact(assetHash: ByteString, pixelHash: ByteString?): List<AnchoredRecord>

    suspend fun byWatermark(captureId: CaptureId): List<AnchoredRecord>

    suspend fun pdq(hashes: List<ByteString>): PdqProbe

    suspend fun derivatives(root: AnchoredRecord): List<AnchoredRecord>

    suspend fun reference(record: AnchoredRecord): ByteString?
}

fun interface VerifyEngine {
    suspend fun judge(input: ByteString, oracle: VerifyOracle, options: JudgeOptions, stages: StageSink): CheckResult
}
