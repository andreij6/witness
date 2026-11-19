package app.witness.core.domain.port

import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Checks
import app.witness.core.model.ClaimDraft
import app.witness.core.model.Edit
import app.witness.core.model.GnssChecks
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import app.witness.core.model.TimeEvidence
import app.witness.core.model.WitnessClaim

data class CaptureFrame(val jpeg: ByteString, val rotationDegrees: Int, val elapsedNs: Long, val bootCount: Int)

fun interface CaptureSource {
    suspend fun capture(): CaptureFrame
}

data class LocationReading(
    val evidence: LocationEvidence,
    val opening: LocationOpening?,
    val gnssTimeMs: Long?,
    val gnss: GnssChecks,
)

fun interface LocationEvidenceSource {
    suspend fun current(): LocationReading?
}

data class PrepareRequest(
    val frame: CaptureFrame,
    val captureId: CaptureId,
    val time: TimeEvidence,
    val location: LocationEvidence,
    val checks: Checks,
)

class PreparedCapture(val token: Long, val draft: ClaimDraft)

data class SealedCapture(val recordId: RecordId, val claim: WitnessClaim, val submission: Submission, val jpeg: ByteString? = null)

interface SealEngine {
    suspend fun prepare(request: PrepareRequest): PreparedCapture

    suspend fun derive(parent: RecordId, edits: List<Edit>, coarse: String?): PreparedCapture

    suspend fun reseal(recordId: RecordId): PreparedCapture

    suspend fun seal(prepared: PreparedCapture, claim: WitnessClaim): SealedCapture
}

fun interface BlurDetector {
    suspend fun detect(recordId: RecordId): List<BlurProposal>
}
