package app.witness.core.icp.cases

import app.witness.core.icp.id.CaseId

class SealedCase(val link: CaseLink, val meta: CaseMeta, val chunks: List<ByteArray>)

class CaseSealer(private val crypto: CaseCrypto) {
    suspend fun seal(caseId: CaseId, bundle: ByteArray, expiresMs: Long): SealedCase {
        val size = bundle.size.toLong()
        require(CaseChunks.fits(size)) { "bundle of $size bytes exceeds the case limit" }
        val key = crypto.randomBytes(CaseChunks.KEY_BYTES)
        val hash = crypto.sha256(bundle)
        val count = CaseChunks.count(size)
        val chunks = List(count) { ix ->
            val from = CaseChunks.start(ix).toInt()
            val aad = CaseAad.build(caseId, ix, count, hash)
            crypto.sealChunk(key, crypto.randomBytes(CaseChunks.NONCE_BYTES), bundle, from, from + CaseChunks.plainSize(ix, size), aad)
        }
        return SealedCase(CaseLink(caseId, key), CaseMeta(count, size, hash, expiresMs), chunks)
    }
}
