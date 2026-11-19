package app.witness.feature.proof

import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.CheckStrength
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.SecurityLevel
import app.witness.core.ui.Strength
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ItemFactsTest {
    private val time = TimeText.UTC
    private val record = AnchoredRecord(2_108_732, Fixtures.recordId, Fixtures.claim(), Fixtures.T0 + 258_000)

    @Test
    fun headerFollowsAnchoringAndOrigin() {
        assertEquals(ItemHeader(WitnessState.Anchored, "Sealed & anchored", "Unaltered since capture"), ItemFacts.header(Fixtures.capture()))
        assertEquals(WitnessState.Queued, ItemFacts.header(Fixtures.capture(status = AnchoringStatus.QUEUED)).state)
        assertEquals(WitnessState.Queued, ItemFacts.header(Fixtures.capture(status = AnchoringStatus.SEALED)).state)
        assertEquals("Anchoring failed", ItemFacts.header(Fixtures.capture(status = AnchoringStatus.FAILED)).title)
        val import = ItemFacts.header(Fixtures.capture(origin = CaptureOrigin.IMPORT, status = AnchoringStatus.UNVERIFIED))
        assertEquals(WitnessState.Unverified, import.state)
        assertEquals("Not captured with Witness", import.subtitle)
    }

    @Test
    fun derivativeSubtitleListsDeclaredEdits() {
        val h = ItemFacts.header(Fixtures.capture(edits = listOf(Fixtures.blur(), Fixtures.blur(BlurClass.FACE), Edit.Tone(1, 1, 1), Edit.Resize(10, 10))))
        assertEquals("Declared edits: 2 blurs, tone", h.subtitle)
        assertEquals("Unaltered since capture", ItemFacts.header(Fixtures.capture(edits = listOf(Edit.Resize(1, 1)))).subtitle)
    }

    @Test
    fun custodyWithRegistryRecordUsesEvidenceStrengths() {
        val steps = ItemFacts.custody(Fixtures.capture(), record, Fixtures.key(), Fixtures.opening, time)
        assertEquals(listOf("Captured", "Key attested", "Location", "Time", "Anchored"), steps.map { it.title })
        assertEquals("2:12:00 PM · Android phone · Witness", steps[0].detail)
        assertNull(steps[0].strength)
        assertEquals("StrongBox · Google root · not revoked", steps[1].detail)
        assertEquals(Strength.Strong, steps[1].strength)
        assertEquals("Raw satellites agree · ±6 m", steps[2].detail)
        assertEquals(Strength.Strong, steps[2].strength)
        assertEquals("GNSS-signed · clock within 1 s", steps[3].detail)
        assertEquals(Strength.Strong, steps[3].strength)
        assertEquals("2:16:18 PM · Internet Computer", steps[4].detail)
        assertNull(steps[4].strength)
    }

    @Test
    fun custodyOfflineFallsBackToCaptureFacts() {
        val steps = ItemFacts.custody(Fixtures.capture(status = AnchoringStatus.QUEUED, anchor = null, confidence = LocationConfidence.MEDIUM), null, null, null, time)
        assertTrue(steps[1].detail.startsWith("Hardware key · "))
        assertNull(steps[1].strength)
        assertEquals("Fused fix · no satellite check", steps[2].detail)
        assertEquals(Strength.Medium, steps[2].strength)
        assertEquals("Phone clock", steps[3].detail)
        assertEquals("Queued · anchors when you're online", steps[4].detail)
    }

    @Test
    fun custodyReflectsWeakSignals() {
        val weak = record.copy(claim = Fixtures.claim(gnssMs = null, security = SecurityLevel.TEE, reasons = EnumBits.of(LocationReason.MOCK_LOCATION), confidence = LocationConfidence.LOW))
        val steps = ItemFacts.custody(Fixtures.capture(), weak, Fixtures.key(SecurityLevel.TEE, Provisioning.FACTORY), null, time)
        assertEquals("TEE · Factory key · not revoked", steps[1].detail)
        assertEquals(Strength.Medium, steps[1].strength)
        assertEquals("Mock location detected", steps[2].detail)
        assertEquals(Strength.Weak, steps[2].strength)
        assertEquals("Phone clock · no satellite time", steps[3].detail)
        assertEquals(Strength.Medium, steps[3].strength)
    }

    @Test
    fun clockSkewRoundsUpToWholeSeconds() {
        val skewed = record.copy(claim = Fixtures.claim(gnssMs = Fixtures.T0 + 2_500))
        assertEquals("GNSS-signed · clock within 3 s", ItemFacts.custody(Fixtures.capture(), skewed, null, null, time)[3].detail)
    }

    @Test
    fun noFixLocationIsUnavailable() {
        val noFix = record.copy(claim = Fixtures.claim(reasons = EnumBits.of(LocationReason.NO_FIX)))
        val step = ItemFacts.custody(Fixtures.capture(), noFix, null, null, time)[2]
        assertEquals("No location fix", step.detail)
        assertNull(step.strength)
    }

    @Test
    fun strengthMapping() {
        assertEquals(Strength.Strong, ItemFacts.strength(CheckStrength.STRONG))
        assertEquals(Strength.Medium, ItemFacts.strength(CheckStrength.MODERATE))
        assertEquals(Strength.Weak, ItemFacts.strength(CheckStrength.WEAK))
        assertNull(ItemFacts.strength(CheckStrength.UNAVAILABLE))
    }

    @Test
    fun technicalRecordListsIdentifiersInOrder() {
        val fields = ItemFacts.technical(Fixtures.capture(), record, Fixtures.key())
        assertEquals(listOf("record_id", "capture_id", "asset_hash", "pixel_hash", "key_id", "Witness cert serial", "anchored", "MMR leaf"), fields.map { it.key })
        assertEquals(Fixtures.recordId.text, fields[0].display)
        assertEquals("0x" + Fixtures.captureId.value.toString(16).uppercase(), fields[1].copy)
        assertEquals("sha256 9f9f…9f9f", fields[2].display)
        assertEquals("sha256:" + "9f".repeat(32), fields[2].copy)
        assertTrue(fields[4].display.endsWith("StrongBox · RKP"))
        assertEquals(Fixtures.keyId.text, fields[4].copy)
        assertEquals("2026-11-18 14:16:18 UTC", fields[6].display)
        assertEquals("2,108,733", fields[7].display)
        assertEquals("2108733", fields[7].copy)
    }

    @Test
    fun technicalRecordWithoutRegistryUsesLocalAnchor() {
        val fields = ItemFacts.technical(Fixtures.capture(), null, null)
        assertEquals(listOf("record_id", "capture_id", "key_id", "anchored", "MMR leaf"), fields.map { it.key })
        val unanchored = ItemFacts.technical(Fixtures.capture(anchor = null, origin = CaptureOrigin.IMPORT), null, null)
        assertEquals(listOf("record_id", "capture_id"), unanchored.map { it.key })
    }

    @Test
    fun jsonIsValidAndUsesCopyValues() {
        val fields = ItemFacts.technical(Fixtures.capture(), record, Fixtures.key())
        val obj = Json.parseToJsonElement(ItemFacts.json(fields)).jsonObject
        assertEquals(Fixtures.recordId.text, obj["record_id"]!!.jsonPrimitive.content)
        assertEquals("03A1FF", obj["Witness_cert_serial"]!!.jsonPrimitive.content)
        assertEquals(ItemFacts.json(fields), ItemFacts.json(fields))
    }

    @Test
    fun coordinatesUseHemispheresAndFourDecimals() {
        assertEquals("39.7392° N, 104.9903° W · ±6 m", ItemFacts.coordinates(Fixtures.opening))
        assertEquals("33.8688° S, 151.2093° E · ±0 m", ItemFacts.coordinates(LocationOpening(Fixtures.opening.salt, -338_688_000, 1_512_093_000, 50)))
    }

    @Test
    fun degreesRoundAndCarry() {
        assertEquals("0.0000", ItemFacts.degrees(0))
        assertEquals("40.0000", ItemFacts.degrees(399_999_999))
        assertEquals("12.3457", ItemFacts.degrees(123_456_789))
        assertEquals("90.0000", ItemFacts.degrees(-900_000_000))
        assertEquals("214.7484", ItemFacts.degrees(Int.MIN_VALUE))
    }

    @Test
    fun locationProofCarriesOpeningAndCommit() {
        val obj = Json.parseToJsonElement(ItemFacts.locationProof(Fixtures.capture(), Fixtures.opening, record)).jsonObject
        assertEquals("witness/location-opening/v1", obj["type"]!!.jsonPrimitive.content)
        assertEquals("397392000", obj["lat_e7"]!!.jsonPrimitive.content)
        assertEquals("01".repeat(32), obj["salt"]!!.jsonPrimitive.content)
        assertEquals("03".repeat(32), obj["commit"]!!.jsonPrimitive.content)
        val noCommit = Json.parseToJsonElement(ItemFacts.locationProof(Fixtures.capture(), Fixtures.opening, null)).jsonObject
        assertNull(noCommit["commit"])
    }

    @Test
    fun labelsCoverEveryEnum() {
        assertEquals(listOf("StrongBox", "TEE", "Dev key"), SecurityLevel.entries.map(ItemFacts::securityLabel))
        assertEquals(listOf("Google root", "Factory key", "Dev attestation"), Provisioning.entries.map(ItemFacts::provisioningLabel))
        assertEquals(listOf("face", "plate", "house number", "area"), BlurClass.entries.map(ItemFacts::blurNoun))
        assertEquals(3, LocationConfidence.entries.map(ItemFacts::confidenceLabel).distinct().size)
    }
}
