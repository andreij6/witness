package app.witness.core.icp

fun hex(s: String): ByteArray {
    val clean = s.filterNot { it.isWhitespace() }
    return ByteArray(clean.length / 2) { clean.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
}

fun ByteArray.toHex(): String = joinToString("") { (it.toInt() and 0xff).toString(16).padStart(2, '0') }

class XorShift(seed: Long) {
    private var s = seed

    fun next(): Long {
        s = s xor (s shl 13)
        s = s xor (s ushr 7)
        s = s xor (s shl 17)
        return s
    }

    fun below(n: Int): Int = ((next() ushr 1) % n).toInt()

    fun bytes(n: Int): ByteArray {
        var x = next().toInt() or 1
        return ByteArray(n) {
            x = x xor (x shl 13)
            x = x xor (x ushr 17)
            x = x xor (x shl 5)
            x.toByte()
        }
    }
}
