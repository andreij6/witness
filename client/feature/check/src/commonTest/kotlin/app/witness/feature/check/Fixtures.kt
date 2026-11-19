package app.witness.feature.check

import app.witness.core.model.Anchor
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CheckResult
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationReason
import app.witness.core.model.MatchEvidence
import app.witness.core.model.RecordId
import app.witness.core.model.Reference
import app.witness.core.model.Region
import app.witness.core.model.SecurityLevel
import app.witness.core.model.TimeEvidence
import app.witness.core.model.Verdict
import app.witness.core.model.WitnessClaim

object Fixtures {
    const val NOV_18_2026_1412_UTC: Long = 1_795_011_120_000
    val recordId: RecordId = RecordId.fromBytes(byteArrayOf(0, 1, 2, 3, 4, 5, 6, 7, 8, 9))
    val keyId: KeyId = KeyId.fromBytes(ByteArray(16).also { it.fill(7) })

    fun claim(
        dims: Dims = Dims(1080, 1440),
        kind: ClaimKind = ClaimKind.Original,
        coarse: String? = "9xj64",
        security: SecurityLevel = SecurityLevel.STRONG_BOX,
        wallMs: Long = NOV_18_2026_1412_UTC,
    ) = WitnessClaim(
        CaptureId.of(42, 1)!!, kind, ByteString.of(ByteArray(32) { 1 }), ByteString.of(ByteArray(32) { 2 }), ByteString.EMPTY, dims,
        TimeEvidence(wallMs, wallMs, 0, 1),
        LocationEvidence(ByteString.of(ByteArray(32) { 3 }), LocationConfidence.HIGH, EnumBits.of(LocationReason.RAW_GNSS_PASSED)),
        Checks(security, BootState.VERIFIED, true, false, EnumBits.none(), 10),
        KeyRef(keyId, 3), "03A1", coarse,
    )

    fun reference(
        evidence: MatchEvidence = MatchEvidence.PIXELS,
        compared: Int = Region.Q16_ONE,
        changed: Int = 0,
        claim: WitnessClaim = claim(),
    ) = Reference(recordId, claim, Anchor(9, NOV_18_2026_1412_UTC + 60_000), evidence, comparedQ16 = compared, changedQ16 = changed)

    fun result(verdict: Verdict, reference: Reference? = reference(), declared: List<Edit> = emptyList(), regions: List<Region> = emptyList()) =
        CheckResult(verdict, declared = declared, regions = regions, reference = reference)

    fun image(vararg bytes: Byte, name: String? = "IMG_1.jpg", dims: Dims? = null) =
        PickedImage(ByteString.of(byteArrayOf(0xFF.toByte(), 0xD8.toByte(), 0xFF.toByte()) + bytes), name, dims)
}
