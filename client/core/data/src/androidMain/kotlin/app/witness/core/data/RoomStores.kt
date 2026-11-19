package app.witness.core.data

import app.witness.core.data.codec.OpeningCodec
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.OutboxStore
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.TemplateId
import app.witness.core.model.Verdict
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

class RoomCaptureStore(db: WitnessDatabase, private val cipher: BlobCipher, private val images: ImageFiles) : CaptureStore {
    private val dao = db.captureDao()

    override fun observeAll(): Flow<List<Capture>> = dao.observeAll().map { rows -> rows.mapFast { it.toCapture() } }

    override suspend fun get(recordId: RecordId): Capture? = dao.get(recordId.text)?.toCapture()

    override suspend fun insert(capture: Capture, outbox: OutboxEntry?, opening: LocationOpening?, image: ByteString?) {
        require(outbox == null || outbox.recordId == capture.recordId) { "outbox entry belongs to another record" }
        image?.let { images.write(capture.recordId, it) }
        dao.insert(capture.toRow(), outbox?.toEntity(), opening?.let { LocOpeningEntity(capture.recordId.text, cipher.seal(OpeningCodec.encode(it))) })
    }

    override suspend fun replace(old: RecordId, capture: Capture, outbox: OutboxEntry, image: ByteString?) {
        require(outbox.recordId == capture.recordId) { "outbox entry belongs to another record" }
        image?.let { images.write(capture.recordId, it) }
        dao.replace(old.text, capture.toRow(), outbox.toEntity())
        if (old == capture.recordId) return
        if (image == null) images.rename(old, capture.recordId) else images.delete(old)
    }

    override suspend fun update(capture: Capture) = dao.update(capture.toRow())

    override suspend fun setStatus(recordId: RecordId, status: AnchoringStatus, anchor: Anchor?) =
        dao.setStatus(recordId.text, status, anchor?.recordIx, anchor?.anchoredAtMs)

    override suspend fun derivativesOf(recordId: RecordId): List<Capture> = dao.derivativesOf(recordId.text).mapFast { it.toCapture() }

    override suspend fun opening(recordId: RecordId): LocationOpening? =
        dao.opening(recordId.text)?.let { OpeningCodec.decode(cipher.open(it)) }

    override suspend fun readImage(recordId: RecordId): ByteString? = images.read(recordId)

    override suspend fun delete(recordId: RecordId) {
        dao.delete(recordId.text)
        images.delete(recordId)
    }

    suspend fun writeImage(recordId: RecordId, jpeg: ByteString) = images.write(recordId, jpeg)
}

class RoomOutboxStore(db: WitnessDatabase) : OutboxStore {
    private val dao = db.outboxDao()

    override fun observe(): Flow<List<OutboxEntry>> = dao.observe().map { rows -> rows.mapFast { it.toEntry() } }

    override suspend fun all(): List<OutboxEntry> = dao.all().mapFast { it.toEntry() }

    override suspend fun update(entry: OutboxEntry) = dao.put(entry.toEntity())

    override suspend fun remove(recordId: RecordId) = dao.remove(recordId.text)
}

class RoomCaseStore(db: WitnessDatabase) : CaseStore {
    private val dao = db.caseDao()

    override fun observeAll(): Flow<List<Case>> = dao.observeAll().map { rows -> rows.mapFast { it.toCase() } }

    override suspend fun get(id: LocalCaseId): Case? = dao.get(id.value)?.toCase()

    override suspend fun create(title: String, templateId: TemplateId, nowMs: Long): Case {
        val draft = Case(LocalCaseId(0), title, templateId, nowMs)
        return draft.copy(id = LocalCaseId(dao.insert(draft.toEntity())))
    }

    override suspend fun update(case: Case) = dao.update(case.toEntity())

    override suspend fun delete(id: LocalCaseId) = dao.delete(id.value)
}

data class CheckRecord(val checkedAtMs: Long, val verdict: Verdict, val recordId: RecordId?, val label: String?)

class CheckHistoryStore(db: WitnessDatabase, private val keep: Int = DEFAULT_KEEP) {
    private val dao = db.checkHistoryDao()

    init {
        require(keep > 0) { "keep must be positive" }
    }

    suspend fun record(check: CheckRecord) =
        dao.record(CheckHistoryEntity(0, check.checkedAtMs, check.verdict, check.recordId?.text, check.label), keep)

    fun observeRecent(limit: Int): Flow<List<CheckRecord>> = dao.observeRecent(limit.coerceIn(0, keep)).map { rows ->
        rows.mapFast { CheckRecord(it.checkedAtMs, it.verdict, it.recordId?.let(RecordId::parse), it.label) }
    }

    suspend fun clear() = dao.clear()

    companion object {
        const val DEFAULT_KEEP = 50
    }
}
