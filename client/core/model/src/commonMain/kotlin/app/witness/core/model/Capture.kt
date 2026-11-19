package app.witness.core.model

enum class AnchoringStatus { SEALED, QUEUED, ANCHORED, UNVERIFIED, FAILED }

enum class CaptureMode { POST, CASE }

enum class CaptureOrigin { ORIGINAL, DERIVATIVE, IMPORT }

data class Capture(
    val recordId: RecordId,
    val captureId: CaptureId?,
    val origin: CaptureOrigin,
    val status: AnchoringStatus,
    val createdAtMs: Long,
    val keyRef: KeyRef?,
    val locationConfidence: LocationConfidence,
    val dims: Dims,
    val caseId: LocalCaseId? = null,
    val parent: RecordId? = null,
    val edits: List<Edit> = emptyList(),
    val anchor: Anchor? = null,
    val label: String? = null,
    val place: String? = null,
    val caption: String? = null,
    val altText: String? = null,
    val publicErased: Boolean = false,
) {
    val isVerified: Boolean get() = origin != CaptureOrigin.IMPORT
}

data class Submission(
    val recordId: RecordId,
    val keyRef: KeyRef,
    val isOriginal: Boolean,
    val wallMs: Long,
    val claimCbor: ByteString,
    val c2paClaimCbor: ByteString,
    val coseSign1: ByteString,
    val fingerprints: ByteString? = null,
)

enum class OutboxState { PENDING, AWAITING_ENROLLMENT, AWAITING_RESEAL, FAILED }

data class OutboxEntry(
    val submission: Submission,
    val attempts: Int = 0,
    val notBeforeMs: Long = 0,
    val state: OutboxState = OutboxState.PENDING,
    val lastError: String? = null,
) {
    val recordId: RecordId get() = submission.recordId
    val keyId: KeyId get() = submission.keyRef.keyId
    val counter: Long get() = submission.keyRef.counter
}
