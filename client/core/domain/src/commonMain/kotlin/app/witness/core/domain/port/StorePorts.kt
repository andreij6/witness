package app.witness.core.domain.port

import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.Entitlement
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.Settings
import app.witness.core.model.TemplateId
import kotlinx.coroutines.flow.Flow

interface CaptureStore {
    fun observeAll(): Flow<List<Capture>>

    suspend fun get(recordId: RecordId): Capture?

    suspend fun insert(capture: Capture, outbox: OutboxEntry?, opening: LocationOpening?, image: ByteString? = null)

    suspend fun replace(old: RecordId, capture: Capture, outbox: OutboxEntry, image: ByteString? = null)

    suspend fun update(capture: Capture)

    suspend fun setStatus(recordId: RecordId, status: AnchoringStatus, anchor: Anchor?)

    suspend fun derivativesOf(recordId: RecordId): List<Capture>

    suspend fun opening(recordId: RecordId): LocationOpening?

    suspend fun readImage(recordId: RecordId): ByteString?

    suspend fun delete(recordId: RecordId)
}

interface OutboxStore {
    fun observe(): Flow<List<OutboxEntry>>

    suspend fun all(): List<OutboxEntry>

    suspend fun update(entry: OutboxEntry)

    suspend fun remove(recordId: RecordId)
}

interface CaseStore {
    fun observeAll(): Flow<List<Case>>

    suspend fun get(id: LocalCaseId): Case?

    suspend fun create(title: String, templateId: TemplateId, nowMs: Long): Case

    suspend fun update(case: Case)

    suspend fun delete(id: LocalCaseId)
}

interface SettingsStore {
    val settings: Flow<Settings>

    suspend fun update(transform: (Settings) -> Settings): Settings
}

fun interface EntitlementSource {
    fun entitlement(): Flow<Entitlement>
}

fun interface AnchorScheduler {
    fun requestAnchoring()
}
