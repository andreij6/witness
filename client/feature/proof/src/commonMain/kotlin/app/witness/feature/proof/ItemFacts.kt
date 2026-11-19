package app.witness.feature.proof

import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.usecase.ProofFacts
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.Capture
import app.witness.core.model.CheckStrength
import app.witness.core.model.Edit
import app.witness.core.model.EditClass
import app.witness.core.model.EvidenceKind
import app.witness.core.model.KeyInfo
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.SecurityLevel
import app.witness.core.ui.CustodyStep
import app.witness.core.ui.Formatters
import app.witness.core.ui.Strength
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlin.math.abs

@Immutable
data class ItemHeader(val state: WitnessState, val title: String, val subtitle: String)

@Immutable
data class TechField(val key: String, val display: String, val copy: String)

object ItemFacts {
    fun header(capture: Capture): ItemHeader {
        val subtitle = when {
            !capture.isVerified -> "Not captured with Witness"
            capture.edits.any { it.editClass != EditClass.PRESENTATION } -> "Declared edits: " + editSummary(capture.edits)
            else -> "Unaltered since capture"
        }
        return when {
            !capture.isVerified -> ItemHeader(WitnessState.Unverified, "Unverified import", subtitle)
            capture.status == AnchoringStatus.ANCHORED -> ItemHeader(WitnessState.Anchored, "Sealed & anchored", subtitle)
            capture.status == AnchoringStatus.FAILED -> ItemHeader(WitnessState.Altered, "Anchoring failed", "Sealed on this phone · the registry refused it")
            else -> ItemHeader(WitnessState.Queued, "Sealed · waiting to anchor", subtitle)
        }
    }

    fun editSummary(edits: List<Edit>): String {
        val blurs = edits.count { it is Edit.Blur }
        val others = edits.filter { it.editClass == EditClass.GLOBAL }.map(::editNoun).distinct()
        val parts = ArrayList<String>(others.size + 1)
        if (blurs > 0) parts += Formatters.plural(blurs, "blur")
        parts += others
        return parts.joinToString(", ")
    }

    fun editNoun(edit: Edit): String = when (edit) {
        is Edit.Crop -> "crop"
        is Edit.Rotate -> "straighten"
        is Edit.Tone -> "tone"
        is Edit.Filter -> "filter"
        is Edit.Blur -> "blur"
        is Edit.Resize -> "resize"
        is Edit.Recompress -> "recompress"
        is Edit.Orient -> "orientation"
    }

    fun custody(capture: Capture, record: AnchoredRecord?, key: KeyInfo?, opening: LocationOpening?, time: TimeText): List<CustodyStep> {
        val claim = record?.claim
        val anchor = record?.let { app.witness.core.model.Anchor(it.recordIx, it.anchoredAtMs) } ?: capture.anchor
        val evidence = claim?.let { ProofFacts.evidence(it, anchor, key) }
        fun evidenceOf(kind: EvidenceKind): Strength? = evidence?.firstOrNull { it.kind == kind }?.strength?.let(::strength)
        val captured = claim?.time?.wallMs ?: capture.createdAtMs
        val keyDetail = when {
            key != null -> Formatters.joinDot(securityLabel(key.security), provisioningLabel(key.provisioning), if (key.status.isTrusted) "not revoked" else "revoked")
            claim != null -> Formatters.joinDot(securityLabel(claim.checks.keySecurity), "awaiting registry")
            capture.keyRef != null -> "Hardware key · ${capture.keyRef!!.keyId.short()}"
            else -> "No Witness key"
        }
        val locationDetail = Formatters.joinDot(
            claim?.let { locationLabel(it.location.reasons, it.location.confidence) } ?: confidenceLabel(capture.locationConfidence),
            opening?.let { Formatters.accuracy(it.accuracyCm / 100) },
        )
        val timeDetail = claim?.time?.let { t ->
            val skew = t.clockSkewMs
            if (t.gnssMs != null && skew != null) "GNSS-signed · clock within ${maxOf(1, (skew + 999) / 1000)} s" else "Phone clock · no satellite time"
        } ?: "Phone clock"
        return listOf(
            CustodyStep(WitnessIcon.PhotoCamera, "Captured", "${time.timeSeconds(captured)} · Android phone · Witness"),
            CustodyStep(WitnessIcon.Key, "Key attested", keyDetail, evidenceOf(EvidenceKind.HARDWARE_KEY)),
            CustodyStep(
                WitnessIcon.SatelliteAlt,
                "Location",
                locationDetail,
                evidenceOf(EvidenceKind.LOCATION) ?: if (claim == null) strength(confidenceStrength(capture.locationConfidence)) else null,
            ),
            CustodyStep(WitnessIcon.Schedule, "Time", timeDetail, evidenceOf(EvidenceKind.TIME)),
            CustodyStep(
                WitnessIcon.Verified,
                "Anchored",
                anchor?.let { "${time.timeSeconds(it.anchoredAtMs)} · Internet Computer" } ?: "Queued · anchors when you're online",
            ),
        )
    }

    fun strength(s: CheckStrength): Strength? = when (s) {
        CheckStrength.STRONG -> Strength.Strong
        CheckStrength.MODERATE -> Strength.Medium
        CheckStrength.WEAK -> Strength.Weak
        CheckStrength.UNAVAILABLE -> null
    }

    fun confidenceStrength(c: LocationConfidence): CheckStrength = when (c) {
        LocationConfidence.HIGH -> CheckStrength.STRONG
        LocationConfidence.MEDIUM -> CheckStrength.MODERATE
        LocationConfidence.LOW -> CheckStrength.WEAK
    }

    fun confidenceLabel(c: LocationConfidence): String = when (c) {
        LocationConfidence.HIGH -> "Raw satellites agree"
        LocationConfidence.MEDIUM -> "Fused fix · no satellite check"
        LocationConfidence.LOW -> "Weak or inconsistent signals"
    }

    fun locationLabel(reasons: app.witness.core.model.LocationReasons, confidence: LocationConfidence): String = when {
        LocationReason.NO_FIX in reasons -> "No location fix"
        LocationReason.MOCK_LOCATION in reasons -> "Mock location detected"
        LocationReason.RAW_GNSS_PASSED in reasons -> "Raw satellites agree"
        LocationReason.FUSED_FIX in reasons -> "Fused fix · no satellite check"
        else -> confidenceLabel(confidence)
    }

    fun securityLabel(level: SecurityLevel): String = when (level) {
        SecurityLevel.STRONG_BOX -> "StrongBox"
        SecurityLevel.TEE -> "TEE"
        SecurityLevel.DEV -> "Dev key"
    }

    fun provisioningLabel(p: Provisioning): String = when (p) {
        Provisioning.RKP -> "Google root"
        Provisioning.FACTORY -> "Factory key"
        Provisioning.DEV -> "Dev attestation"
    }

    fun technical(capture: Capture, record: AnchoredRecord?, key: KeyInfo?): List<TechField> {
        val claim = record?.claim
        val out = ArrayList<TechField>(8)
        out += TechField("record_id", capture.recordId.text, capture.recordId.text)
        (claim?.captureId ?: capture.captureId)?.let { id ->
            val hex = "0x" + id.value.toString(16).uppercase()
            out += TechField("capture_id", Formatters.abbreviateMiddle(hex, 6, 3), hex)
        }
        claim?.let {
            out += hashField("asset_hash", it.assetHash.hex())
            out += hashField("pixel_hash", it.pixelHash.hex())
        }
        val keyRef = claim?.keyRef ?: capture.keyRef
        keyRef?.let { ref ->
            val id = ref.keyId.text
            val labels = key?.let { listOf(securityLabel(it.security), it.provisioning.name) } ?: emptyList()
            out += TechField("key_id", Formatters.joinDot(listOf(Formatters.abbreviateMiddle(id)) + labels), id)
        }
        claim?.certSerial?.let { out += TechField("Witness cert serial", Formatters.abbreviateMiddle(it, 5, 0), it) }
        val anchoredAt = record?.anchoredAtMs ?: capture.anchor?.anchoredAtMs
        anchoredAt?.let { out += TechField("anchored", TimeText.utc(it), TimeText.utc(it)) }
        val ix = record?.recordIx ?: capture.anchor?.recordIx
        ix?.let { out += TechField("MMR leaf", Formatters.groupThousands(it + 1), (it + 1).toString()) }
        return out
    }

    private fun hashField(key: String, hex: String) = TechField(key, Formatters.shortHash("sha256", hex), "sha256:$hex")

    fun json(fields: List<TechField>): String = buildJsonObject { fields.forEach { put(it.key.replace(' ', '_'), it.copy) } }.toString()

    fun coordinates(opening: LocationOpening): String {
        val lat = opening.latE7
        val lon = opening.lonE7
        return "${degrees(lat)}° ${if (lat >= 0) "N" else "S"}, ${degrees(lon)}° ${if (lon >= 0) "E" else "W"} · ${Formatters.accuracy(opening.accuracyCm / 100)}"
    }

    fun degrees(e7: Int): String {
        val v = abs(e7.toLong())
        val whole = v / 10_000_000
        val frac = (v % 10_000_000 + 500) / 1000
        val carry = frac / 10_000
        val f = (frac % 10_000).toString().padStart(4, '0')
        return "${whole + carry}.$f"
    }

    fun locationProof(capture: Capture, opening: LocationOpening, record: AnchoredRecord?): String = buildJsonObject {
        put("type", "witness/location-opening/v1")
        put("record_id", capture.recordId.text)
        put("lat_e7", opening.latE7)
        put("lon_e7", opening.lonE7)
        put("accuracy_cm", opening.accuracyCm)
        put("salt", opening.salt.hex())
        record?.claim?.location?.commit?.let { put("commit", it.hex()) }
    }.toString()

    fun blurNoun(c: BlurClass): String = when (c) {
        BlurClass.FACE -> "face"
        BlurClass.PLATE -> "plate"
        BlurClass.HOUSE_NO -> "house number"
        BlurClass.OTHER -> "area"
    }
}
