package app.witness.core.domain.fake

import app.witness.core.domain.Clock
import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.EraseOutcome
import app.witness.core.domain.port.EraseRequest
import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.PdqPosting
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ProbeResult
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.ShardMap
import app.witness.core.domain.port.SubmitResult
import app.witness.core.domain.port.UploadRequest
import app.witness.core.domain.port.UploadTicket
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CaseId
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.KeyRef
import app.witness.core.model.Provisioning
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import app.witness.core.model.WitnessClaim

class FakeRegistry(
    private val clock: Clock,
    private val claims: (RecordId) -> WitnessClaim? = { null },
) : RegistryGateway {
    var offline: Boolean = false
    var rejectEnrollment: String? = null
    var windowMs: Long = 60L * 86_400_000
    val scripted = ArrayDeque<SubmitResult>()
    val submitted = ArrayList<List<Submission>>()
    val erased = ArrayList<EraseRequest>()
    val enrollments = ArrayList<EnrollmentRequest>()
    var shardMap: ShardMap = ShardMap(listOf(ShardInfo(0, 0)), listOf(ShardInfo(0, 0)))
    var getCalls: Int = 0
        private set

    private val keys = HashMap<KeyId, KeyInfo>()
    private val records = ArrayList<AnchoredRecord>()
    private val byId = HashMap<RecordId, AnchoredRecord>()
    private val byCounter = HashMap<KeyRef, AnchoredRecord>()

    override suspend fun challenge(): ByteString = online { ByteString.of("challenge-${clock.nowEpochMillis()}".encodeToByteArray()) }

    override suspend fun enroll(request: EnrollmentRequest): EnrollOutcome = online {
        enrollments += request
        rejectEnrollment?.let { return@online EnrollOutcome.Rejected(it) }
        val now = clock.nowEpochMillis()
        val info = KeyInfo(request.keyId, app.witness.core.model.SecurityLevel.STRONG_BOX, Provisioning.RKP, request.freshness, now, now + windowMs, KeyHistory.enrolled(now))
        keys[request.keyId] = info
        EnrollOutcome.Enrolled(info, ByteString.of("cert".encodeToByteArray()), "1a${keys.size}")
    }

    override suspend fun submit(batch: List<Submission>): List<SubmitResult> = online {
        require(batch.size <= RegistryGateway.MAX_BATCH) { "batch too large" }
        submitted += batch
        batch.map { s -> scripted.removeFirstOrNull() ?: accept(s) }
    }

    private fun accept(s: Submission): SubmitResult {
        byCounter[s.keyRef]?.let { return SubmitResult.Duplicate(Anchor(it.recordIx, it.anchoredAtMs)) }
        val key = keys[s.keyRef.keyId] ?: return SubmitResult.Rejected("unknown_key")
        if (!key.status.isTrusted) return SubmitResult.Rejected("key_revoked")
        if (s.isOriginal && s.wallMs > key.activeUntilMs) return SubmitResult.KeyExpired
        val claim = claims(s.recordId) ?: return SubmitResult.Rejected("unknown_claim")
        val rec = AnchoredRecord(records.size.toLong(), s.recordId, claim, clock.nowEpochMillis())
        records += rec
        byId[rec.recordId] = rec
        byCounter[s.keyRef] = rec
        return SubmitResult.Anchored(Anchor(rec.recordIx, rec.anchoredAtMs))
    }

    override suspend fun get(recordId: RecordId): AnchoredRecord? = online {
        getCalls++
        byId[recordId]
    }

    override suspend fun getByIndex(recordIx: Long): AnchoredRecord? = online { records.getOrNull(recordIx.toInt()) }

    override suspend fun getKey(keyId: KeyId): KeyInfo? = online { keys[keyId] }

    override suspend fun byCapture(captureId: CaptureId): List<RecordId> = online {
        records.filter { it.claim.captureId == captureId }.take(32).map { it.recordId }
    }

    override suspend fun shards(): ShardMap = online { shardMap }

    override suspend fun eraseMine(request: EraseRequest): EraseOutcome = online {
        val rec = byId[request.recordId] ?: return@online EraseOutcome.Rejected("unknown_record")
        if (rec.claim.keyRef.keyId != request.keyId) return@online EraseOutcome.Rejected("wrong_key")
        erased += request
        EraseOutcome.Erased
    }

    fun putKey(info: KeyInfo) {
        keys[info.keyId] = info
    }

    fun putRecord(recordId: RecordId, claim: WitnessClaim, anchoredAtMs: Long = clock.nowEpochMillis()): AnchoredRecord {
        val rec = AnchoredRecord(records.size.toLong(), recordId, claim, anchoredAtMs)
        records += rec
        byId[recordId] = rec
        byCounter[claim.keyRef] = rec
        return rec
    }

    fun record(recordId: RecordId): AnchoredRecord? = byId[recordId]

    private inline fun <T> online(block: () -> T): T {
        if (offline) throw OfflineException()
        return block()
    }
}

class FakeIndex : IndexGateway {
    val pdq = HashMap<Int, MutableList<Pair<ByteString, PdqPosting>>>()
    val exact = HashMap<Int, MutableMap<ByteString, MutableList<Long>>>()
    val derivatives = HashMap<Long, MutableList<Long>>()
    val down = HashSet<Int>()
    val truncatedShards = HashSet<Int>()
    var probes: Int = 0
        private set

    override suspend fun probe(shard: ShardInfo, query: ProbeQuery): ProbeResult {
        probes++
        if (shard.id in down) throw OfflineException("shard ${shard.id} down")
        val hits = pdq[shard.id].orEmpty().filter { (h, _) -> h in query.pdq }.map { it.second }
        val ex = exact[shard.id].orEmpty()
        return ProbeResult(hits, query.exact.flatMap { ex[it].orEmpty() }, shard.id in truncatedShards)
    }

    override suspend fun derivatives(shard: ShardInfo, rootIx: Long): List<Long> {
        if (shard.id in down) throw OfflineException("shard ${shard.id} down")
        return derivatives[rootIx].orEmpty()
    }

    fun addPdq(shard: Int, hash: ByteString, recordIx: Long, distance: Int) {
        pdq.getOrPut(shard) { ArrayList() }.add(hash to PdqPosting(recordIx, distance))
    }

    fun addExact(shard: Int, hash: ByteString, recordIx: Long) {
        exact.getOrPut(shard) { HashMap() }.getOrPut(hash) { ArrayList() }.add(recordIx)
    }
}

class FakeMedia : MediaGateway {
    class Upload(val request: UploadRequest, val chunks: MutableMap<Int, ByteArray> = HashMap(), var committed: Boolean = false)

    val uploads = LinkedHashMap<String, Upload>()
    val references = HashMap<RecordId, ByteString>()
    var failPutAt: Int? = null
    var caseExpiryMs: Long? = null
    override var maxCaseBytes: Long = Long.MAX_VALUE

    override suspend fun begin(request: UploadRequest): UploadTicket {
        val id = "u${uploads.size}"
        uploads[id] = Upload(request)
        return UploadTicket(request.caseId.shard, id)
    }

    override suspend fun putChunk(ticket: UploadTicket, index: Int, bytes: ByteArray) {
        if (failPutAt == index) throw OfflineException("put failed")
        require(bytes.size <= MediaGateway.MAX_CHUNK_BYTES) { "chunk too large" }
        val u = uploads.getValue(ticket.uploadId)
        require(index in 0 until u.request.chunkCount) { "chunk index out of range" }
        u.chunks[index] = bytes
    }

    override suspend fun commit(ticket: UploadTicket) {
        val u = uploads.getValue(ticket.uploadId)
        check(u.chunks.size == u.request.chunkCount) { "missing chunks" }
        u.committed = true
    }

    override suspend fun reference(recordId: RecordId): ByteString? = references[recordId]

    override suspend fun caseExpiry(caseId: CaseId): Long? =
        caseExpiryMs?.takeIf { uploads.values.any { it.committed && it.request.caseId == caseId } }
}
