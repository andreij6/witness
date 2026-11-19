package app.witness.core.data.codec

import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare

object ShareCodec {
    const val VERSION = 1
    const val MAX_SHARES = 256

    fun encode(shares: List<CaseShare>): ByteArray? {
        if (shares.isEmpty()) return null
        require(shares.size <= MAX_SHARES) { "too many shares" }
        val w = ByteWriter(8 + shares.size * 96).byte(VERSION).int(shares.size)
        for (s in shares) w.string(s.caseId.text).string(s.url).long(s.createdAtMs).nullableLong(s.expiresAtMs).int(s.itemCount)
        return w.toByteArray()
    }

    fun decode(bytes: ByteArray?): List<CaseShare> {
        if (bytes == null) return emptyList()
        val r = ByteReader(bytes)
        require(r.byte() == VERSION) { "unsupported share version" }
        val n = r.int()
        require(n in 0..MAX_SHARES) { "too many shares" }
        val out = ArrayList<CaseShare>(n)
        repeat(n) { out += CaseShare(CaseId.parse(r.string(64)), r.string(4096), r.long(), r.nullableLong(), r.int()) }
        require(r.exhausted) { "trailing bytes" }
        return out
    }
}
