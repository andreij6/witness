package app.witness.core.domain.fake

import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.CounterStore
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.port.SettingsStore
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.Entitlement
import app.witness.core.model.KeyId
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.Settings
import app.witness.core.model.TemplateId
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.flow.updateAndGet

class InMemoryOutboxStore : OutboxStore {
    private val state = MutableStateFlow<Map<RecordId, OutboxEntry>>(emptyMap())

    override fun observe(): Flow<List<OutboxEntry>> = state.map { it.values.sortedWith(ORDER) }

    override suspend fun all(): List<OutboxEntry> = state.value.values.sortedWith(ORDER)

    override suspend fun update(entry: OutboxEntry) = state.update { it + (entry.recordId to entry) }

    override suspend fun remove(recordId: RecordId) = state.update { it - recordId }

    operator fun get(recordId: RecordId): OutboxEntry? = state.value[recordId]

    val size: Int get() = state.value.size

    private companion object {
        val ORDER = compareBy<OutboxEntry>({ it.keyId }, { it.counter })
    }
}

class InMemoryCaptureStore(private val outbox: InMemoryOutboxStore = InMemoryOutboxStore()) : CaptureStore {
    private val state = MutableStateFlow<Map<RecordId, Capture>>(emptyMap())
    private val openings = HashMap<RecordId, LocationOpening>()
    private val images = HashMap<RecordId, ByteString>()

    override fun observeAll(): Flow<List<Capture>> = state.map { it.values.sortedByDescending(Capture::createdAtMs) }

    override suspend fun get(recordId: RecordId): Capture? = state.value[recordId]

    override suspend fun insert(capture: Capture, outbox: OutboxEntry?, opening: LocationOpening?, image: ByteString?) {
        opening?.let { openings[capture.recordId] = it }
        image?.let { images[capture.recordId] = it }
        outbox?.let { this.outbox.update(it) }
        state.update { it + (capture.recordId to capture) }
    }

    override suspend fun replace(old: RecordId, capture: Capture, outbox: OutboxEntry, image: ByteString?) {
        openings.remove(old)?.let { openings[capture.recordId] = it }
        val previous = images.remove(old)
        (image ?: previous)?.let { images[capture.recordId] = it }
        this.outbox.remove(old)
        this.outbox.update(outbox)
        state.update { it - old + (capture.recordId to capture) }
    }

    override suspend fun update(capture: Capture) = state.update { if (capture.recordId in it) it + (capture.recordId to capture) else it }

    override suspend fun setStatus(recordId: RecordId, status: AnchoringStatus, anchor: Anchor?) = state.update { m ->
        m[recordId]?.let { m + (recordId to it.copy(status = status, anchor = anchor ?: it.anchor)) } ?: m
    }

    override suspend fun derivativesOf(recordId: RecordId): List<Capture> {
        val children = HashMap<RecordId, MutableList<Capture>>()
        for (c in state.value.values) c.parent?.let { children.getOrPut(it) { ArrayList() }.add(c) }
        val out = ArrayList<Capture>()
        val stack = ArrayDeque(children[recordId].orEmpty())
        while (stack.isNotEmpty()) {
            val c = stack.removeLast()
            out += c
            children[c.recordId]?.let(stack::addAll)
        }
        return out.sortedBy { it.createdAtMs }
    }

    override suspend fun opening(recordId: RecordId): LocationOpening? = openings[recordId]

    override suspend fun readImage(recordId: RecordId): ByteString? = images[recordId]

    override suspend fun delete(recordId: RecordId) {
        state.update { it - recordId }
        images.remove(recordId)
        openings.remove(recordId)
        outbox.remove(recordId)
    }

    fun putImage(recordId: RecordId, jpeg: ByteString) {
        images[recordId] = jpeg
    }

    fun putOpening(recordId: RecordId, opening: LocationOpening) {
        openings[recordId] = opening
    }

    val snapshot: Map<RecordId, Capture> get() = state.value
}

class InMemoryCaseStore : CaseStore {
    private val state = MutableStateFlow<Map<LocalCaseId, Case>>(emptyMap())
    private var nextId = 1L

    override fun observeAll(): Flow<List<Case>> = state.map { it.values.sortedByDescending(Case::updatedAtMs) }

    override suspend fun get(id: LocalCaseId): Case? = state.value[id]

    override suspend fun create(title: String, templateId: TemplateId, nowMs: Long): Case {
        val case = Case(LocalCaseId(nextId++), title, templateId, nowMs)
        state.update { it + (case.id to case) }
        return case
    }

    override suspend fun update(case: Case) = state.update { if (case.id in it) it + (case.id to case) else it }

    override suspend fun delete(id: LocalCaseId) = state.update { it - id }
}

class InMemorySettingsStore(initial: Settings = Settings()) : SettingsStore {
    private val state = MutableStateFlow(initial)

    override val settings: Flow<Settings> get() = state

    override suspend fun update(transform: (Settings) -> Settings): Settings = state.updateAndGet(transform)

    val value: Settings get() = state.value
}

class FakeEntitlementSource(initial: Entitlement = Entitlement.FREE) : EntitlementSource {
    val state = MutableStateFlow(initial)

    override fun entitlement(): Flow<Entitlement> = state
}

class InMemoryCounterStore : CounterStore {
    private val next = HashMap<KeyId, Long>()

    override suspend fun reserve(keyId: KeyId): Long {
        val v = next[keyId] ?: 0L
        next[keyId] = v + 1
        return v
    }

    fun skip(keyId: KeyId, by: Long) {
        next[keyId] = (next[keyId] ?: 0L) + by
    }
}
