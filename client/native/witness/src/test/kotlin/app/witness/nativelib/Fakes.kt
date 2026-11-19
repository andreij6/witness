package app.witness.nativelib

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.model.ByteString
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import kotlinx.coroutines.awaitCancellation
import uniffi.witness_android.Begun
import uniffi.witness_android.BootState
import uniffi.witness_android.CertifiedKey
import uniffi.witness_android.CertifiedRecord
import uniffi.witness_android.Checks
import uniffi.witness_android.ClaimView
import uniffi.witness_android.Confidence
import uniffi.witness_android.Dims
import uniffi.witness_android.Enrolled
import uniffi.witness_android.Enrollment
import uniffi.witness_android.EntitlementGrant
import uniffi.witness_android.Fresh
import uniffi.witness_android.KeyInfo
import uniffi.witness_android.KeyStatus
import uniffi.witness_android.KeystoreSigner
import uniffi.witness_android.Kind
import uniffi.witness_android.LocationSpec
import uniffi.witness_android.PreparedHandleInterface
import uniffi.witness_android.ProbeResult
import uniffi.witness_android.Prov
import uniffi.witness_android.SealedCapture
import uniffi.witness_android.Security
import uniffi.witness_android.SecurityLevel
import uniffi.witness_android.ShardInfo
import uniffi.witness_android.Shards
import uniffi.witness_android.StatusChange
import uniffi.witness_android.Submission
import uniffi.witness_android.SubmitResult
import uniffi.witness_android.TimeEvidence
import uniffi.witness_android.Trustmark
import uniffi.witness_android.UploadTarget
import uniffi.witness_android.VerdictView
import uniffi.witness_android.WitnessException
import uniffi.witness_android.WitnessInterface
import java.nio.file.Files
import java.util.Base64

object Vectors {
    val CERT: ByteArray = Base64.getDecoder().decode(
        "MIIBTjCB9qADAgECAgIBAjAKBggqhkjOPQQDAjAXMRUwEwYDVQQDDAx3aXRuZXNzLXRlc3QwHhcNMjYxMTE5MTA0MzA3WhcNMzYxMTE2MTA0MzA3WjAXMRUwEwYDVQQDDAx3aXRuZXNzLXRlc3QwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQGHNc83782yb5YISTNbpADLx98BIRNXwQEOhV/4HMBG1aPmfHkh8jdOSPxXlO1QaDOxAnr7dYm423jtGdP/7qYozIwMDAdBgNVHQ4EFgQU8Roys9qRH6cVo6WMIb68kLzYzbgwDwYDVR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNHADBEAiBdFtrisjpaTC1HX8WTLHNK/0beNFpKWMWkH7FPRZ5mEgIgLxlwsiBqpWqLAqxbf7ip0EvI1KP8A8wMGfLUgcfXfc0=",
    )
    val CA: ByteArray = byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x07)
    const val NS: Long = 1_000_000

    fun claimView(keyId: ByteArray, counter: Long = 4, kind: Kind = Kind.Original, wallMs: Long = 1_700_000_000_000) = ClaimView(
        captureId = 0x00AB_CDEFuL,
        kind = kind,
        assetHash = ByteArray(32) { 1 },
        pixelHash = ByteArray(32) { 2 },
        fpCommit = ByteArray(32) { 3 },
        dims = Dims(640u, 480u),
        time = TimeEvidence(wallMs.toULong(), (wallMs + 5).toULong(), 77uL, 3u),
        location = LocationSpec(ByteArray(32) { 9 }, Confidence.MEDIUM, 0b101u),
        checks = Checks(SecurityLevel.TEE, BootState.VERIFIED, true, false, 0b11u, 0u),
        keyId = keyId,
        counter = counter.toULong(),
        certSerial = byteArrayOf(1, 2),
        coarse = "9xj64",
    )

    fun keyInfo(keyId: ByteArray, prov: Prov = Prov.DEV, security: Security = Security.TEE) = KeyInfo(
        keyId = keyId,
        pubkey = ByteArray(65),
        security = security,
        provisioning = prov,
        freshness = Fresh.FRESH,
        patch = 202611u,
        enrolledNs = (10 * NS).toULong(),
        activeUntilNs = (99 * NS).toULong(),
        history = listOf(StatusChange(KeyStatus.ACTIVE, (10 * NS).toULong())),
        chainSerials = emptyList(),
    )
}

class FakeWitness : WitnessInterface {
    var caRoot: ByteArray? = Vectors.CA
    var caRootCalls = 0
    var challenge: ByteArray = byteArrayOf(1, 2, 3)
    var enrollAnswer: () -> Enrolled = { error("no enroll answer") }
    val enrollments = ArrayList<Enrollment>()
    val keys = HashMap<String, KeyInfo>()
    var submitAnswer: (List<Submission>) -> List<SubmitResult> = { emptyList() }
    var records = HashMap<String, CertifiedRecord>()
    var byCaptureAnswer: List<String> = emptyList()
    var shards = Shards(listOf(ShardInfo(0u, "index-0", true)), listOf(ShardInfo(0u, "media-0", true), ShardInfo(1u, "media-1", false)))
    var eraseFailure: WitnessException? = null
    val erased = ArrayList<Pair<String, ByteArray>>()
    val erasedSigners = ArrayList<KeystoreSigner>()
    var probeAnswer = ProbeResult(emptyList(), emptyList(), emptyList(), false, 0uL)
    val probes = ArrayList<Triple<List<ByteArray>, List<ULong>, List<ByteArray>>>()
    val begun = ArrayList<Pair<UploadTarget, ByteArray>>()
    val beganShards = ArrayList<UByte>()
    var missing: List<UInt> = listOf(0u)
    val chunks = ArrayList<Pair<ULong, ByteArray>>()
    val commits = ArrayList<ULong>()
    var checkAnswer: () -> VerdictView = { error("no check answer") }
    var failure: WitnessException? = null
    var hang: Boolean = false

    private suspend fun fail() {
        if (hang) awaitCancellation()
        failure?.let { throw it }
    }

    override suspend fun beginUpload(shard: UByte, target: UploadTarget, size: ULong, sha256: ByteArray, keyId: ByteArray, signer: KeystoreSigner, grant: EntitlementGrant?): Begun {
        fail()
        begun += target to sha256
        beganShards += shard
        return Begun(missing, 40uL + begun.size.toULong())
    }

    override suspend fun byCapture(captureId: ULong): List<String> = byCaptureAnswer.also { fail() }

    override suspend fun caRoot(): ByteArray? {
        fail()
        caRootCalls++
        return caRoot
    }

    override suspend fun challenge(): ByteArray = challenge.also { fail() }

    override suspend fun check(image: ByteArray, watermark: Trustmark?): VerdictView {
        fail()
        return checkAnswer()
    }

    override suspend fun commit(shard: UByte, upload: ULong): String {
        fail()
        commits += upload
        return "/c/$upload"
    }

    override suspend fun enroll(enrollment: Enrollment): Enrolled {
        fail()
        enrollments += enrollment
        return enrollAnswer()
    }

    override suspend fun eraseMine(recordId: String, nonce: ByteArray, signer: KeystoreSigner) {
        fail()
        eraseFailure?.let { throw it }
        erased += recordId to nonce
        erasedSigners += signer
    }

    override suspend fun get(recordId: String): CertifiedRecord? = records[recordId].also { fail() }

    override suspend fun getKey(keyId: ByteArray): CertifiedKey? {
        fail()
        return keys[ByteString.of(keyId).hex()]?.let { CertifiedKey(it, ByteArray(0), ByteArray(0)) }
    }

    override suspend fun probe(pdq: List<ByteArray>, watermarks: List<ULong>, exact: List<ByteArray>): ProbeResult {
        fail()
        probes += Triple(pdq, watermarks, exact)
        return probeAnswer
    }

    override suspend fun putChunk(shard: UByte, upload: ULong, ix: UInt, bytes: ByteArray) {
        fail()
        chunks += upload to bytes
    }

    override suspend fun shards(): Shards = shards.also { fail() }

    override suspend fun submit(batch: List<Submission>): List<SubmitResult> {
        fail()
        return submitAnswer(batch)
    }
}

class FakeHandle(private val sealed: SealedCapture) : PreparedHandleInterface, AutoCloseable {
    var closed = false
    var finishedWith: Triple<ULong, KeystoreSigner, ByteArray>? = null

    override fun assetHash(): ByteArray = sealed.claim.assetHash

    override fun finish(counter: ULong, signer: KeystoreSigner, caRoot: ByteArray): SealedCapture {
        finishedWith = Triple(counter, signer, caRoot)
        return sealed.copy(claim = sealed.claim.copy(counter = counter))
    }

    override fun height(): UInt = sealed.claim.dims.height

    override fun pixelHash(): ByteArray = sealed.claim.pixelHash

    override fun watermarked(): Boolean = false

    override fun width(): UInt = sealed.claim.dims.width

    override fun close() {
        closed = true
    }
}

class Harness {
    val clock = FakeClock()
    val vault = FakeKeyVault(clock)
    val witness = FakeWitness()
    val dir = Files.createTempDirectory("native").toFile()
    val trust = NativeTrust(witness, java.io.File(dir, "ca.der"))
    val certs = HashMap<KeyId, ByteString>()
    val keys = NativeKeys(vault, trust) { certs[it] }

    suspend fun enrolledKey(): KeyId {
        val k = vault.generate(ByteString.of(byteArrayOf(1)), Freshness.FRESH).key
        vault.markEnrolled(k.keyId, 1, Long.MAX_VALUE, "0102", ByteString.of(Vectors.CERT))
        certs[k.keyId] = ByteString.of(Vectors.CERT)
        return k.keyId
    }
}

fun variantsOf(type: Class<*>): Int = type.declaredClasses.count { it != type && type.isAssignableFrom(it) }
