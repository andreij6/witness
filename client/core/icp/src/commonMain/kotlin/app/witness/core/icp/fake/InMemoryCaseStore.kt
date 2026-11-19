package app.witness.core.icp.fake

import app.witness.core.icp.cases.CaseMeta
import app.witness.core.icp.cases.CaseStore
import app.witness.core.icp.cases.CaseUploader
import app.witness.core.icp.cases.SealedCase
import app.witness.core.icp.id.CaseId
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class InMemoryCaseStore : CaseStore, CaseUploader {
    private class Stored(val meta: CaseMeta, val chunks: List<ByteArray>)

    private val lock = Mutex()
    private val cases = HashMap<CaseId, Stored>()

    override suspend fun upload(sealed: SealedCase) {
        lock.withLock { cases[sealed.link.caseId] = Stored(sealed.meta, sealed.chunks.map { it.copyOf() }) }
    }

    override suspend fun meta(caseId: CaseId): CaseMeta? = lock.withLock { cases[caseId]?.meta }

    override suspend fun chunk(caseId: CaseId, ix: Int): ByteArray? =
        lock.withLock { cases[caseId]?.chunks?.getOrNull(ix)?.copyOf() }

    suspend fun erase(caseId: CaseId): Boolean = lock.withLock { cases.remove(caseId) != null }
}
