package app.witness.core.icp.witness

import app.witness.core.model.AnchoredRecord
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.ImageFormat
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.KeyRef
import app.witness.core.model.KeyStatus
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.Provisioning
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.StatusChange
import app.witness.core.model.TimeEvidence
import app.witness.core.model.WitnessClaim
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.longOrNull

class WireException(message: String) : Exception(message)

object WireJson {
    const val NS_PER_MS: Long = 1_000_000

    val UNKNOWN_CHECKS: Checks = Checks(SecurityLevel.TEE, BootState.VERIFIED, true, false, EnumBits.none(), 0)

    fun parse(text: String): JsonObject = try {
        Json.parseToJsonElement(text) as? JsonObject
    } catch (_: IllegalArgumentException) {
        null
    } ?: throw WireException("not a JSON object")

    fun record(o: JsonObject): AnchoredRecord = AnchoredRecord(
        recordIx = o.u64("recordIx"),
        recordId = o.recordId("recordId"),
        claim = claim(o.obj("claim")),
        anchoredAtMs = o.u64("anchoredNs") / NS_PER_MS,
    )

    fun claim(o: JsonObject): WitnessClaim {
        val location = o.objOrNull("location")
        val checks = o.objOrNull("checks")
        return WitnessClaim(
            captureId = CaptureId.fromLong(o.u64("captureId")) ?: throw WireException("captureId"),
            kind = kind(o.obj("kind")),
            assetHash = o.hex("assetHash"),
            pixelHash = o.hex("pixelHash"),
            fpCommit = o.hexOrNull("fpCommit") ?: ByteString.EMPTY,
            dims = Dims(o.int("width"), o.int("height")),
            time = TimeEvidence(o.u64("wallMs"), o.u64OrNull("gnssMs"), o.u64OrNull("elapsedNs") ?: 0, o.intOrNull("bootCount") ?: 0),
            location = location?.let {
                LocationEvidence(it.hexOrNull("commit"), it.code("confidence", LocationConfidence.entries), EnumBits(it.int("reasons")))
            } ?: LocationEvidence.NONE,
            checks = checks?.let {
                Checks(
                    keySecurity = it.code("keySecurity", SecurityLevel.entries),
                    boot = it.code("boot", BootState.entries),
                    deviceLocked = it.bool("deviceLocked"),
                    mockLocation = it.bool("mockLocation"),
                    gnss = EnumBits(it.int("gnss")),
                    recapture = it.int("recapture"),
                )
            } ?: UNKNOWN_CHECKS,
            keyRef = KeyRef(o.keyId("keyId"), o.u64("counter")),
            certSerial = o.strOrNull("certSerial")?.takeIf { it.isNotEmpty() },
            coarse = o.strOrNull("coarse"),
        )
    }

    fun kind(o: JsonObject): ClaimKind = when (val type = o.str("type")) {
        "original" -> ClaimKind.Original
        "derivative" -> ClaimKind.Derivative(
            parent = o.recordId("parent"),
            root = CaptureId.fromLong(o.u64("root")) ?: throw WireException("root"),
            edits = o.array("edits").map { edit(it.asObject()) },
        )
        else -> throw WireException("claim kind $type")
    }

    fun edit(o: JsonObject): Edit = when (val kind = o.str("kind")) {
        "resize" -> Edit.Resize(o.int("w"), o.int("h"), o.int("filter"))
        "recompress" -> Edit.Recompress(name(o.str("format"), ImageFormat.entries), o.int("quality"))
        "orient" -> Edit.Orient(o.int("orientation"))
        "crop" -> Edit.Crop(rect(o))
        "rotate" -> Edit.Rotate(o.int("degE2"))
        "tone" -> Edit.Tone(o.int("exposureE2"), o.int("contrastE2"), o.int("saturationE2"))
        "filter" -> Edit.Filter(o.int("id"), o.int("strengthE2"))
        "blur" -> Edit.Blur(rect(o), name(o.str("class"), BlurClass.entries), o.int("sigmaE2"))
        else -> throw WireException("edit kind $kind")
    }

    fun key(o: JsonObject): KeyInfo {
        val enrolled = o.u64("enrolledNs") / NS_PER_MS
        val changes = o.array("history").map {
            val c = it.asObject()
            StatusChange(variant(c.str("status"), KeyStatus.entries), c.u64("sinceNs") / NS_PER_MS)
        }
        return KeyInfo(
            keyId = o.keyId("keyId"),
            security = variant(o.str("security"), SecurityLevel.entries),
            provisioning = variant(o.str("provisioning"), Provisioning.entries),
            freshness = variant(o.str("freshness"), Freshness.entries),
            enrolledAtMs = enrolled,
            activeUntilMs = o.u64("activeUntilNs") / NS_PER_MS,
            history = KeyHistory.of(changes) ?: throw WireException("key history"),
        )
    }

    fun rect(o: JsonObject): Rect = Rect(o.int("x"), o.int("y"), o.int("w"), o.int("h"))

    fun <E : Enum<E>> name(text: String, entries: List<E>): E =
        entries.firstOrNull { it.name.equals(text, ignoreCase = true) } ?: throw WireException("unknown $text")

    fun <E : Enum<E>> variant(text: String, entries: List<E>): E =
        entries.firstOrNull { it.name.replace("_", "").equals(text, ignoreCase = true) } ?: throw WireException("unknown $text")

    private fun <E : Enum<E>> JsonObject.code(name: String, entries: List<E>): E =
        entries.getOrNull(int(name)) ?: throw WireException("$name out of range")

    fun JsonElement.asObject(): JsonObject = this as? JsonObject ?: throw WireException("expected an object")

    fun JsonObject.obj(name: String): JsonObject = objOrNull(name) ?: throw WireException("missing $name")

    fun JsonObject.objOrNull(name: String): JsonObject? = this[name] as? JsonObject

    fun JsonObject.array(name: String): JsonArray = this[name] as? JsonArray ?: throw WireException("missing $name")

    private fun JsonObject.primitive(name: String): JsonPrimitive? = (this[name] as? JsonPrimitive)?.takeUnless { it is JsonNull }

    fun JsonObject.strOrNull(name: String): String? = primitive(name)?.takeIf { it.isString }?.content

    fun JsonObject.str(name: String): String = strOrNull(name) ?: throw WireException("missing $name")

    fun JsonObject.intOrNull(name: String): Int? = primitive(name)?.takeUnless { it.isString }?.intOrNull

    fun JsonObject.int(name: String): Int = intOrNull(name) ?: throw WireException("missing $name")

    fun JsonObject.bool(name: String): Boolean = primitive(name)?.takeUnless { it.isString }?.booleanOrNull ?: throw WireException("missing $name")

    fun JsonObject.u64OrNull(name: String): Long? {
        val p = primitive(name) ?: return null
        val v = (if (p.isString) p.content.takeIf { s -> s.isNotEmpty() && s.all { it in '0'..'9' } }?.toLongOrNull() else p.longOrNull)
        return v?.takeIf { it >= 0 } ?: throw WireException("$name is not a u63")
    }

    fun JsonObject.u64(name: String): Long = u64OrNull(name) ?: throw WireException("missing $name")

    fun JsonObject.hexOrNull(name: String): ByteString? = strOrNull(name)?.let {
        try {
            ByteString.ofHex(it)
        } catch (_: IllegalArgumentException) {
            throw WireException("$name is not hex")
        }
    }

    fun JsonObject.hex(name: String): ByteString = hexOrNull(name) ?: throw WireException("missing $name")

    fun JsonObject.keyId(name: String): KeyId = try {
        KeyId.fromBytes(hex(name).toByteArray())
    } catch (_: IllegalArgumentException) {
        throw WireException("$name is not a key id")
    }

    fun JsonObject.recordId(name: String): RecordId = RecordId.parseOrNull(str(name)) ?: throw WireException("$name is not a record id")
}
