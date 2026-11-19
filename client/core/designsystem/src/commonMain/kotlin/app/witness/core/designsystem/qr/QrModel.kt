package app.witness.core.designsystem.qr

enum class QrEcc(internal val formatBits: Int) { L(1), M(0), Q(3), H(2) }

enum class QrMode(internal val indicator: Int, private val countBits: IntArray) {
    Numeric(0x1, intArrayOf(10, 12, 14)),
    Alphanumeric(0x2, intArrayOf(9, 11, 13)),
    Byte(0x4, intArrayOf(8, 16, 16));

    internal fun characterCountBits(version: Int): Int = countBits[(version + 7) / 17]
}

class QrMatrix internal constructor(
    val version: Int,
    val ecc: QrEcc,
    val mask: Int,
    val mode: QrMode,
    private val modules: BooleanArray,
) {
    val size: Int = QrTables.size(version)

    operator fun get(x: Int, y: Int): Boolean = modules[y * size + x]

    fun darkModules(): Int = modules.count { it }

    override fun equals(other: Any?): Boolean =
        other is QrMatrix && other.version == version && other.ecc == ecc && other.mask == mask && other.modules.contentEquals(modules)

    override fun hashCode(): Int = modules.contentHashCode() * 31 + version

    override fun toString(): String = buildString((size + 1) * size) {
        for (y in 0 until size) {
            for (x in 0 until size) append(if (get(x, y)) '#' else '.')
            if (y < size - 1) append('\n')
        }
    }
}
