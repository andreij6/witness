package app.witness.core.icp.index

class ShardProbe(
    val answered: Boolean,
    val truncated: Boolean,
    val indexedUpto: Long,
    val localIx: IntArray,
    val shape: IntArray,
    val distance: IntArray,
    val watermark: LongArray,
    val exact: LongArray,
) {
    init {
        require(localIx.size == shape.size && shape.size == distance.size) { "pdq hit columns differ in length" }
    }

    val pdqCount: Int get() = localIx.size

    fun localIxAt(i: Int): Long = localIx[i].toLong() and 0xffff_ffffL

    override fun equals(other: Any?): Boolean =
        this === other || other is ShardProbe &&
            answered == other.answered &&
            truncated == other.truncated &&
            indexedUpto == other.indexedUpto &&
            localIx.contentEquals(other.localIx) &&
            shape.contentEquals(other.shape) &&
            distance.contentEquals(other.distance) &&
            watermark.contentEquals(other.watermark) &&
            exact.contentEquals(other.exact)

    override fun hashCode(): Int =
        ((indexedUpto.hashCode() * 31 + localIx.contentHashCode()) * 31 + watermark.contentHashCode()) * 31 + exact.contentHashCode()

    override fun toString(): String =
        "ShardProbe(answered=$answered, truncated=$truncated, indexedUpto=$indexedUpto, pdq=$pdqCount, watermark=${watermark.size}, exact=${exact.size})"

    companion object {
        private val NONE_INT = IntArray(0)
        private val NONE_LONG = LongArray(0)

        val FAILED: ShardProbe = ShardProbe(false, false, 0, NONE_INT, NONE_INT, NONE_INT, NONE_LONG, NONE_LONG)

        fun answered(
            indexedUpto: Long,
            truncated: Boolean = false,
            localIx: IntArray = NONE_INT,
            shape: IntArray = NONE_INT,
            distance: IntArray = NONE_INT,
            watermark: LongArray = NONE_LONG,
            exact: LongArray = NONE_LONG,
        ): ShardProbe = ShardProbe(true, truncated, indexedUpto, localIx, shape, distance, watermark, exact)
    }
}
