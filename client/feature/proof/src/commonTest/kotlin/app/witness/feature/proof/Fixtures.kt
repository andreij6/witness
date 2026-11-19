package app.witness.feature.proof

import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureId
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.KeyRef
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.RecordId
import app.witness.core.model.Rect
import app.witness.core.model.SecurityLevel
import app.witness.core.model.TimeEvidence
import app.witness.core.model.WitnessClaim

object Fixtures {
    const val T0: Long = 1_795_011_120_000
    val recordId: RecordId = RecordId.fromBytes(byteArrayOf(0, 1, 2, 3, 4, 5, 6, 7, 8, 9))
    val otherId: RecordId = RecordId.fromBytes(byteArrayOf(9, 8, 7, 6, 5, 4, 3, 2, 1, 0))
    val thirdId: RecordId = RecordId.fromBytes(byteArrayOf(5, 5, 5, 5, 5, 5, 5, 5, 5, 5))
    val keyId: KeyId = KeyId.fromBytes(ByteArray(16).also { it.fill(7) })
    val captureId: CaptureId = CaptureId.of(0x1F3A9C2, 1)!!

    fun blur(c: BlurClass = BlurClass.PLATE) = Edit.Blur(Rect(0, 0, 10, 10), c, 100)

    fun claim(
        kind: ClaimKind = ClaimKind.Original,
        gnssMs: Long? = T0 + 400,
        security: SecurityLevel = SecurityLevel.STRONG_BOX,
        reasons: app.witness.core.model.LocationReasons = EnumBits.of(LocationReason.RAW_GNSS_PASSED),
        confidence: LocationConfidence = LocationConfidence.HIGH,
        coarse: String? = "9xj64",
    ) = WitnessClaim(
        captureId, kind, ByteString.of(ByteArray(32).also { it.fill(0x9f.toByte()) }), ByteString.of(ByteArray(32).also { it.fill(0x41) }), ByteString.EMPTY,
        Dims(3000, 4000),
        TimeEvidence(T0, gnssMs, 0, 1),
        LocationEvidence(ByteString.of(ByteArray(32).also { it.fill(3) }), confidence, reasons),
        Checks(security, BootState.VERIFIED, true, false, EnumBits.none(), 10),
        KeyRef(keyId, 3), "03A1FF", coarse,
    )

    fun key(security: SecurityLevel = SecurityLevel.STRONG_BOX, provisioning: Provisioning = Provisioning.RKP) =
        KeyInfo(keyId, security, provisioning, Freshness.FRESH, 0, T0 * 2, KeyHistory.enrolled(0))

    fun capture(
        id: RecordId = recordId,
        status: AnchoringStatus = AnchoringStatus.ANCHORED,
        origin: CaptureOrigin = CaptureOrigin.ORIGINAL,
        caseId: LocalCaseId? = null,
        createdAtMs: Long = T0,
        edits: List<Edit> = emptyList(),
        anchor: Anchor? = Anchor(2_108_732, T0 + 258_000),
        confidence: LocationConfidence = LocationConfidence.HIGH,
    ) = Capture(
        id, captureId, origin, status, createdAtMs, if (origin == CaptureOrigin.IMPORT) null else KeyRef(keyId, 3), confidence, Dims(3000, 4000),
        caseId = caseId, edits = edits, anchor = anchor,
    )

    val opening = LocationOpening(ByteString.of(ByteArray(32).also { it.fill(1) }), 397_392_000, -1_049_903_000, 600)
}
