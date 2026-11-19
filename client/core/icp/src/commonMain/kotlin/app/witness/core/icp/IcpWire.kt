package app.witness.core.icp

import app.witness.core.icp.id.RecordId
import app.witness.core.icp.registry.ShardInfo

object IcpWire {
    fun u64(text: String): Long {
        if (text.isEmpty() || text.length > 19 || !text.all { it in '0'..'9' }) throw IcpException("not a u64: '$text'")
        return text.toLongOrNull() ?: throw IcpException("u64 '$text' exceeds 63 bits")
    }

    fun recordId(text: String): RecordId = RecordId.parse(text) ?: throw IcpException("malformed record id '$text'")

    fun recordIds(joined: String): List<RecordId> =
        if (joined.isEmpty()) emptyList() else joined.split(',').map(::recordId)

    fun shard(canisterId: String, baseIx: String, indexedUpto: String): ShardInfo {
        if (canisterId.isBlank() || ',' in canisterId) throw IcpException("malformed canister id '$canisterId'")
        return ShardInfo(canisterId, u64(baseIx), u64(indexedUpto))
    }

    fun joinIds(shards: List<ShardInfo>): String = shards.joinToString(",") { it.canisterId }
}
