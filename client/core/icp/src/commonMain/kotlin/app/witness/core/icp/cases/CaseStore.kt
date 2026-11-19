package app.witness.core.icp.cases

import app.witness.core.icp.id.CaseId

class CaseMeta(val count: Int, val bytes: Long, bundleHash: ByteArray, val expiresMs: Long) {
    private val hash = bundleHash.copyOf()

    val bundleHash: ByteArray get() = hash.copyOf()

    fun hashEquals(other: ByteArray): Boolean = hash.contentEquals(other)

    override fun equals(other: Any?): Boolean =
        other is CaseMeta && count == other.count && bytes == other.bytes && expiresMs == other.expiresMs && hash.contentEquals(other.hash)

    override fun hashCode(): Int = (count * 31 + bytes.hashCode()) * 31 + expiresMs.hashCode()

    override fun toString(): String = "CaseMeta(count=$count, bytes=$bytes, expiresMs=$expiresMs)"
}

interface CaseStore {
    suspend fun meta(caseId: CaseId): CaseMeta?

    suspend fun chunk(caseId: CaseId, ix: Int): ByteArray?
}

fun interface CaseUploader {
    suspend fun upload(sealed: SealedCase)
}
