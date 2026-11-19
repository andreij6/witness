package app.witness.core.icp.registry

import app.witness.core.icp.id.RecordId

class CertifiedRecord(
    val recordIx: Long,
    val recordId: RecordId,
    val claim: ByteArray,
    val c2paClaim: ByteArray,
    val cose: ByteArray,
    val anchoredNs: Long,
    val mmrProof: List<ByteArray>,
    val leafCount: Long,
    val certificate: ByteArray,
) {
    override fun equals(other: Any?): Boolean =
        this === other || other is CertifiedRecord &&
            recordIx == other.recordIx &&
            recordId == other.recordId &&
            anchoredNs == other.anchoredNs &&
            leafCount == other.leafCount &&
            claim.contentEquals(other.claim) &&
            c2paClaim.contentEquals(other.c2paClaim) &&
            cose.contentEquals(other.cose) &&
            certificate.contentEquals(other.certificate) &&
            mmrProof.size == other.mmrProof.size &&
            mmrProof.indices.all { mmrProof[it].contentEquals(other.mmrProof[it]) }

    override fun hashCode(): Int = recordIx.hashCode() * 31 + recordId.hashCode()

    override fun toString(): String = "CertifiedRecord(ix=$recordIx, id=$recordId, anchoredNs=$anchoredNs)"
}
