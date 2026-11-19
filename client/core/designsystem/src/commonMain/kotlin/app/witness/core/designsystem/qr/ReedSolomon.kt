package app.witness.core.designsystem.qr

internal object ReedSolomon {
    private val exp = IntArray(512)
    private val log = IntArray(256)

    init {
        var x = 1
        for (i in 0 until 255) {
            exp[i] = x
            log[x] = i
            x = x shl 1
            if (x >= 0x100) x = x xor 0x11D
        }
        for (i in 255 until 512) exp[i] = exp[i - 255]
    }

    fun multiply(a: Int, b: Int): Int = if (a == 0 || b == 0) 0 else exp[log[a] + log[b]]

    fun divisor(degree: Int): IntArray {
        require(degree in 1..255) { "degree out of range: $degree" }
        val result = IntArray(degree)
        result[degree - 1] = 1
        var root = 1
        for (i in 0 until degree) {
            for (j in 0 until degree) {
                result[j] = multiply(result[j], root)
                if (j + 1 < degree) result[j] = result[j] xor result[j + 1]
            }
            root = multiply(root, 2)
        }
        return result
    }

    fun remainder(data: ByteArray, offset: Int, length: Int, divisor: IntArray, out: ByteArray, outOffset: Int) {
        val degree = divisor.size
        val acc = IntArray(degree)
        for (k in offset until offset + length) {
            val factor = (data[k].toInt() and 0xFF) xor acc[0]
            for (i in 0 until degree - 1) acc[i] = acc[i + 1] xor multiply(divisor[i], factor)
            acc[degree - 1] = multiply(divisor[degree - 1], factor)
        }
        for (i in 0 until degree) out[outOffset + i] = acc[i].toByte()
    }
}
