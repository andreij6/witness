package app.witness.core.model

class ByteString private constructor(private val data: ByteArray) : Comparable<ByteString> {
    val size: Int get() = data.size

    operator fun get(index: Int): Byte = data[index]

    fun toByteArray(): ByteArray = data.copyOf()

    fun copyInto(destination: ByteArray, offset: Int = 0): ByteArray = data.copyInto(destination, offset)

    fun hex(): String {
        val out = CharArray(data.size * 2)
        data.forEachIndexed { i, b ->
            val v = b.toInt() and 0xff
            out[2 * i] = HEX[v ushr 4]
            out[2 * i + 1] = HEX[v and 15]
        }
        return out.concatToString()
    }

    override fun equals(other: Any?): Boolean = other is ByteString && data.contentEquals(other.data)

    override fun hashCode(): Int = data.contentHashCode()

    override fun compareTo(other: ByteString): Int {
        val n = minOf(size, other.size)
        for (i in 0 until n) {
            val c = (data[i].toInt() and 0xff) - (other.data[i].toInt() and 0xff)
            if (c != 0) return c
        }
        return size - other.size
    }

    override fun toString(): String = "ByteString(${data.size})"

    companion object {
        private const val HEX = "0123456789abcdef"
        const val SHA256_SIZE: Int = 32
        val EMPTY: ByteString = ByteString(ByteArray(0))

        fun of(bytes: ByteArray): ByteString = ByteString(bytes.copyOf())

        fun wrap(bytes: ByteArray): ByteString = ByteString(bytes)

        fun ofHex(hex: String): ByteString {
            require(hex.length % 2 == 0) { "odd hex length" }
            return ByteString(ByteArray(hex.length / 2) { i -> ((nibble(hex[2 * i]) shl 4) or nibble(hex[2 * i + 1])).toByte() })
        }

        private fun nibble(c: Char): Int = when (c) {
            in '0'..'9' -> c - '0'
            in 'a'..'f' -> c - 'a' + 10
            in 'A'..'F' -> c - 'A' + 10
            else -> throw IllegalArgumentException("invalid hex '$c'")
        }
    }
}
