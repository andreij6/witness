package app.witness.core.model

sealed interface ClaimKind {
    val edits: List<Edit>
    val isPresentationOnly: Boolean get() = edits.isPresentationOnly()

    data object Original : ClaimKind {
        override val edits: List<Edit> get() = emptyList()
    }

    data class Derivative(val parent: RecordId, val root: CaptureId, override val edits: List<Edit>) : ClaimKind
}

data class Dims(val width: Int, val height: Int) {
    val isValid: Boolean get() = width > 0 && height > 0
    val isPortrait: Boolean get() = height > width
}

data class KeyRef(val keyId: KeyId, val counter: Long)

data class ClaimDraft(
    val captureId: CaptureId,
    val kind: ClaimKind,
    val assetHash: ByteString,
    val pixelHash: ByteString,
    val fpCommit: ByteString,
    val dims: Dims,
    val time: TimeEvidence,
    val location: LocationEvidence,
    val checks: Checks,
    val coarse: String? = null,
) {
    fun finalize(keyRef: KeyRef, certSerial: String?): WitnessClaim =
        WitnessClaim(captureId, kind, assetHash, pixelHash, fpCommit, dims, time, location, checks, keyRef, certSerial, coarse)
}

data class WitnessClaim(
    val captureId: CaptureId,
    val kind: ClaimKind,
    val assetHash: ByteString,
    val pixelHash: ByteString,
    val fpCommit: ByteString,
    val dims: Dims,
    val time: TimeEvidence,
    val location: LocationEvidence,
    val checks: Checks,
    val keyRef: KeyRef,
    val certSerial: String?,
    val coarse: String? = null,
) {
    val version: Int get() = VERSION
    val isOriginal: Boolean get() = kind is ClaimKind.Original

    companion object {
        const val VERSION: Int = 2
    }
}

data class AnchoredRecord(
    val recordIx: Long,
    val recordId: RecordId,
    val claim: WitnessClaim,
    val anchoredAtMs: Long,
    val flags: VerdictFlags = EnumBits.none(),
)

data class Anchor(val recordIx: Long, val anchoredAtMs: Long)
