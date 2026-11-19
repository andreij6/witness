package app.witness.core.icp.cases

import app.witness.core.domain.Clock
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope

sealed interface CaseOpenResult {
    class Opened(val bundle: ByteArray) : CaseOpenResult

    data object NotFound : CaseOpenResult

    data object Expired : CaseOpenResult

    data object Tampered : CaseOpenResult
}

class CaseOpener(private val store: CaseStore, private val crypto: CaseCrypto, private val clock: Clock) {
    suspend fun open(link: CaseLink): CaseOpenResult {
        val meta = store.meta(link.caseId) ?: return CaseOpenResult.NotFound
        if (meta.expiresMs <= clock.nowEpochMillis()) return CaseOpenResult.Expired
        if (!CaseChunks.fits(meta.bytes) || meta.count != CaseChunks.count(meta.bytes)) return CaseOpenResult.Tampered
        val key = link.key
        val bundle = ByteArray(meta.bytes.toInt())
        val hash = meta.bundleHash
        val intact = coroutineScope {
            (0 until meta.count).map { ix ->
                async {
                    val chunk = store.chunk(link.caseId, ix) ?: return@async false
                    if (chunk.size != CaseChunks.plainSize(ix, meta.bytes) + CaseChunks.OVERHEAD) return@async false
                    val aad = CaseAad.build(link.caseId, ix, meta.count, hash)
                    val plain = crypto.openChunk(key, chunk, aad) ?: return@async false
                    plain.copyInto(bundle, CaseChunks.start(ix).toInt())
                    true
                }
            }.awaitAll().all { it }
        }
        if (!intact || !meta.hashEquals(crypto.sha256(bundle))) return CaseOpenResult.Tampered
        return CaseOpenResult.Opened(bundle)
    }
}
