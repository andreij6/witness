package app.witness.core.icp.registry

data class ShardInfo(val canisterId: String, val baseIx: Long, val indexedUpto: Long)

class ShardMap private constructor(val index: List<ShardInfo>, val media: List<ShardInfo>) {
    fun indexShardOf(recordIx: Long): ShardInfo? {
        var lo = 0
        var hi = index.size - 1
        var found = -1
        while (lo <= hi) {
            val mid = (lo + hi) ushr 1
            if (index[mid].baseIx <= recordIx) {
                found = mid
                lo = mid + 1
            } else {
                hi = mid - 1
            }
        }
        return if (found < 0) null else index[found]
    }

    fun mediaShard(shard: Int): ShardInfo? = media.getOrNull(shard)

    override fun equals(other: Any?): Boolean = other is ShardMap && index == other.index && media == other.media

    override fun hashCode(): Int = index.hashCode() * 31 + media.hashCode()

    override fun toString(): String = "ShardMap(index=$index, media=$media)"

    companion object {
        val EMPTY: ShardMap = ShardMap(emptyList(), emptyList())

        fun of(index: List<ShardInfo>, media: List<ShardInfo>): ShardMap {
            val sorted = index.sortedBy { it.baseIx }
            for (i in 1 until sorted.size) {
                require(sorted[i].baseIx != sorted[i - 1].baseIx) { "duplicate shard base ${sorted[i].baseIx}" }
            }
            require(sorted.all { it.baseIx >= 0 }) { "negative shard base" }
            return ShardMap(sorted, media.toList())
        }
    }
}
