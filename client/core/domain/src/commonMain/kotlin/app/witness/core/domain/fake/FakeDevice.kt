package app.witness.core.domain.fake

import app.witness.core.domain.Clock
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.CaptureSource
import app.witness.core.domain.port.GeneratedKey
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.LocationReading
import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.port.PreparedCapture
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SealedCapture
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.ClaimDraft
import app.witness.core.model.ClaimKind
import app.witness.core.model.DeviceKey
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import app.witness.core.model.Submission
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import app.witness.core.model.WitnessClaim

class FakeKeyVault(private val clock: Clock, private val security: SecurityLevel = SecurityLevel.STRONG_BOX) : KeyVault {
    private val keys = LinkedHashMap<KeyId, GeneratedKey>()
    private var generated = 0
    val deleted = ArrayList<KeyId>()

    override suspend fun activeKey(): DeviceKey? = keys.values.lastOrNull { !it.key.retired }?.key

    override suspend fun key(keyId: KeyId): DeviceKey? = keys[keyId]?.key

    override suspend fun generate(challenge: ByteString, freshness: Freshness): GeneratedKey {
        val id = KeyId.fromDigest(SoftwareSha256.digest("fake-key-${generated++}".encodeToByteArray()))
        val chain = listOf(ByteString.of(id.toBytes()), ByteString.of("root".encodeToByteArray()))
        return GeneratedKey(DeviceKey(id, security, freshness, clock.nowEpochMillis()), chain, challenge).also { keys[id] = it }
    }

    override suspend fun attestation(keyId: KeyId): GeneratedKey? = keys[keyId]

    override suspend fun markEnrolled(keyId: KeyId, enrolledAtMs: Long, activeUntilMs: Long, certSerial: String, c2paCert: ByteString) {
        edit(keyId) { it.copy(enrolledAtMs = enrolledAtMs, activeUntilMs = activeUntilMs, certSerial = certSerial) }
    }

    override suspend fun retire(keyId: KeyId) = edit(keyId) { it.copy(retired = true) }

    override suspend fun delete(keyId: KeyId) {
        keys.remove(keyId)?.let { deleted += keyId }
    }

    override suspend fun sign(keyId: KeyId, message: ByteArray): ByteString {
        check(keyId in keys) { "unknown key" }
        return ByteString.of(SoftwareSha256.digest(keyId.toBytes() + message))
    }

    fun put(key: DeviceKey) {
        keys[key.keyId] = GeneratedKey(key, emptyList(), ByteString.EMPTY)
    }

    fun forget(keyId: KeyId) {
        keys.remove(keyId)
    }

    private fun edit(keyId: KeyId, f: (DeviceKey) -> DeviceKey) {
        val g = keys[keyId] ?: return
        keys[keyId] = g.copy(key = f(g.key))
    }

    companion object {
        fun verify(keyId: KeyId, message: ByteArray, signature: ByteString): Boolean =
            ByteString.of(SoftwareSha256.digest(keyId.toBytes() + message)) == signature
    }
}

class FakeSealEngine : SealEngine {
    private val drafts = HashMap<Long, ClaimDraft>()
    private val frames = HashMap<Long, ByteString>()
    private val sealed = HashMap<RecordId, WitnessClaim>()
    private var nextToken = 1L
    var prepareCalls: Int = 0
        private set

    override suspend fun prepare(request: PrepareRequest): PreparedCapture {
        prepareCalls++
        val jpeg = request.frame.jpeg.toByteArray()
        val digest = ByteString.of(SoftwareSha256.digest(jpeg))
        val turned = request.frame.rotationDegrees % 180 != 0
        val dims = if (turned) Dims(3024, 4032) else Dims(4032, 3024)
        return token(ClaimDraft(request.captureId, ClaimKind.Original, digest, digest, digest, dims, request.time, request.location, request.checks))
            .also { frames[it.token] = request.frame.jpeg }
    }

    override suspend fun derive(parent: RecordId, edits: List<Edit>, coarse: String?): PreparedCapture {
        val p = sealed[parent] ?: throw IllegalStateException("unknown parent")
        val crop = edits.filterIsInstance<Edit.Crop>().lastOrNull()?.rect
        val dims = crop?.let { Dims(it.w, it.h) } ?: p.dims
        val asset = ByteString.of(SoftwareSha256.digest((p.assetHash.hex() + edits.toString()).encodeToByteArray()))
        return token(ClaimDraft(p.captureId, ClaimKind.Derivative(parent, p.captureId, edits), asset, asset, p.fpCommit, dims, p.time, p.location, p.checks, coarse))
    }

    override suspend fun reseal(recordId: RecordId): PreparedCapture {
        val c = sealed[recordId] ?: throw IllegalStateException("unknown record")
        return token(ClaimDraft(c.captureId, c.kind, c.assetHash, c.pixelHash, c.fpCommit, c.dims, c.time, c.location, c.checks, c.coarse))
    }

    override suspend fun seal(prepared: PreparedCapture, claim: WitnessClaim): SealedCapture {
        check(drafts.remove(prepared.token) != null) { "unknown or reused prepared capture" }
        val jpeg = frames.remove(prepared.token)
        val cbor = encode(claim)
        val id = RecordId.fromDigest(SoftwareSha256.digest(cbor))
        sealed[id] = claim
        val sub = Submission(id, claim.keyRef, claim.isOriginal, claim.time.wallMs, ByteString.of(cbor), ByteString.of(cbor.copyOf(8)), ByteString.of(SoftwareSha256.digest(cbor)))
        return SealedCapture(id, claim, sub, jpeg)
    }

    fun claim(recordId: RecordId): WitnessClaim? = sealed[recordId]

    fun register(recordId: RecordId, claim: WitnessClaim) {
        sealed[recordId] = claim
    }

    private fun token(draft: ClaimDraft): PreparedCapture {
        val t = nextToken++
        drafts[t] = draft
        return PreparedCapture(t, draft)
    }

    private fun encode(c: WitnessClaim): ByteArray =
        listOf(
            c.captureId.value, c.kind.toString(), c.assetHash.hex(), c.pixelHash.hex(), c.dims, c.time, c.location.confidence,
            c.location.commit?.hex(), c.keyRef.keyId.text, c.keyRef.counter, c.certSerial, c.coarse,
        ).joinToString("|").encodeToByteArray()
}

class FakeCaptureSource(var frame: CaptureFrame = CaptureFrame(ByteString.of(byteArrayOf(-1, -40, 1, 2, 3)), 0, 1_000, 1)) : CaptureSource {
    var shots: Int = 0
        private set

    override suspend fun capture(): CaptureFrame {
        shots++
        return frame.copy(jpeg = ByteString.of(frame.jpeg.toByteArray() + shots.toByte()), elapsedNs = frame.elapsedNs + shots)
    }
}

class FakeLocationSource(var reading: LocationReading? = null, var failure: Exception? = null) : LocationEvidenceSource {
    override suspend fun current(): LocationReading? {
        failure?.let { throw it }
        return reading
    }
}

class FakeBlurDetector(var proposals: Map<RecordId, List<BlurProposal>> = emptyMap(), var failure: Exception? = null) : BlurDetector {
    override suspend fun detect(recordId: RecordId): List<BlurProposal> {
        failure?.let { throw it }
        return proposals[recordId].orEmpty()
    }
}

class FakeAssistant(
    var available: Boolean = false,
    var answer: ((AssistantContext, List<TemplateSchema>) -> Suggestion?)? = null,
    var failure: Exception? = null,
) : Assistant {
    val contexts = ArrayList<AssistantContext>()

    override suspend fun isAvailable(): Boolean = available

    override suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? {
        contexts += context
        failure?.let { throw it }
        return answer?.invoke(context, templates)
    }
}

class FakeTemplateCatalog(var templates: List<TemplateSchema> = BUNDLED) : TemplateCatalog {
    override suspend fun templates(): List<TemplateSchema> = templates

    companion object {
        val BUNDLED: List<TemplateSchema> = listOf(
            TemplateSchema(
                TemplateId.CAR_ACCIDENT, "Car accident",
                listOf("car", "crash", "accident", "collision", "fender bender", "bumper", "vehicle"),
                listOf(
                    SlotSpec("other_plate", SlotType.TEXT, maxLength = 16),
                    SlotSpec("insurer", SlotType.TEXT),
                    SlotSpec("impact", SlotType.CHOICE, listOf("front", "rear", "left", "right"), required = true),
                    SlotSpec("injuries", SlotType.FLAG),
                ),
            ),
            TemplateSchema(
                TemplateId.PROPERTY_DAMAGE, "Property damage",
                listOf("damage", "hail", "storm", "flood", "leak", "roof", "fire"),
                listOf(SlotSpec("cause", SlotType.CHOICE, listOf("hail", "water", "fire", "wind", "other")), SlotSpec("notes", SlotType.TEXT)),
            ),
            TemplateSchema(
                TemplateId.RENTAL_CONDITION, "Rental condition",
                listOf("rental", "move out", "move in", "walkthrough", "apartment", "deposit", "lease"),
                listOf(SlotSpec("room", SlotType.TEXT), SlotSpec("move", SlotType.CHOICE, listOf("in", "out"), required = true)),
            ),
            TemplateSchema(TemplateId.GENERAL, "General", emptyList(), listOf(SlotSpec("notes", SlotType.TEXT))),
        )
    }
}
