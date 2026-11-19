package app.witness.core.model

enum class Platform(val topK: Int) {
    ANDROID(8),
    WEB(3),
}

data class Region(val x: Int, val y: Int, val w: Int, val h: Int) {
    val leftFraction: Float get() = x.toFloat() / Q16_ONE
    val topFraction: Float get() = y.toFloat() / Q16_ONE
    val widthFraction: Float get() = w.toFloat() / Q16_ONE
    val heightFraction: Float get() = h.toFloat() / Q16_ONE

    companion object {
        const val Q16_ONE: Int = 1 shl 16

        fun normalized(x: Int, y: Int, w: Int, h: Int, frameWidth: Int, frameHeight: Int): Region? {
            if (frameWidth <= 0 || frameHeight <= 0 || w <= 0 || h <= 0 || x < 0 || y < 0) return null
            val (nx, nw) = axis(x, w, frameWidth)
            val (ny, nh) = axis(y, h, frameHeight)
            return if (nw > 0 && nh > 0) Region(nx, ny, nw, nh) else null
        }

        private fun axis(start: Int, len: Int, total: Int): Pair<Int, Int> {
            val t = total.toLong()
            val lo = start.toLong().coerceAtMost(t)
            val hi = (lo + len).coerceAtMost(t)
            val a = lo * Q16_ONE / t
            val b = (hi * Q16_ONE + t - 1) / t
            return a.toInt() to (b - a).toInt()
        }
    }
}

enum class MatchEvidence { SEAL, EXACT_HASH, PIXELS, GRID }

data class Reference(
    val recordId: RecordId,
    val claim: WitnessClaim,
    val anchor: Anchor?,
    val evidence: MatchEvidence,
    val confidenceQ16: Int = Region.Q16_ONE,
    val comparedQ16: Int = Region.Q16_ONE,
    val changedQ16: Int = 0,
)

data class CheckResult(
    val verdict: Verdict,
    val flags: VerdictFlags = EnumBits.none(),
    val reasons: VerdictReasons = EnumBits.none(),
    val declared: List<Edit> = emptyList(),
    val regions: List<Region> = emptyList(),
    val reference: Reference? = null,
    val version: Int = VERDICT_VERSION,
    val proofsRejected: Boolean = false,
) {
    val registryUnverified: Boolean get() = proofsRejected && verdict == Verdict.NO_RECORD

    companion object {
        const val VERDICT_VERSION: Int = 1
        val NO_RECORD: CheckResult = CheckResult(Verdict.NO_RECORD)
    }
}

enum class CheckStage { READ_FILE, CREDENTIALS, FINGERPRINTS, WATERMARK, COMPARE }

enum class StageState { WAITING, RUNNING, DONE, SKIPPED, FAILED }

data class StageStatus(
    val stage: CheckStage,
    val state: StageState,
    val completed: Int = 0,
    val total: Int = 0,
    val detail: String? = null,
)
