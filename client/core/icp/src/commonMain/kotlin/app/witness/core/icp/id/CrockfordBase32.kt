package app.witness.core.icp.id

enum class Base32Error { LENGTH, SYMBOL, PADDING }

class Base32Exception(val error: Base32Error, val symbol: Char? = null) :
    IllegalArgumentException(if (symbol == null) "base32 ${error.name.lowercase()}" else "invalid base32 symbol '$symbol'")

object CrockfordBase32 {
    const val ALPHABET: String = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

    private const val INVALID: Byte = -1
    private const val SKIP: Byte = -2

    private val decodeTable = ByteArray(128) { INVALID }.also { t ->
        ALPHABET.forEachIndexed { i, c ->
            t[c.code] = i.toByte()
            t[c.lowercaseChar().code] = i.toByte()
        }
        "IiLl".forEach { t[it.code] = 1 }
        "Oo".forEach { t[it.code] = 0 }
        t['-'.code] = SKIP
    }

    fun encodedLength(byteCount: Int): Int = (byteCount * 8 + 4) / 5

    fun encode(bytes: ByteArray): String {
        val out = CharArray(encodedLength(bytes.size))
        var acc = 0
        var bits = 0
        var o = 0
        for (b in bytes) {
            acc = (acc shl 8) or (b.toInt() and 0xff)
            bits += 8
            while (bits >= 5) {
                bits -= 5
                out[o++] = ALPHABET[(acc ushr bits) and 31]
            }
        }
        if (bits > 0) out[o] = ALPHABET[(acc shl (5 - bits)) and 31]
        return out.concatToString()
    }

    fun symbolValue(c: Char): Int = if (c.code < 128) decodeTable[c.code].toInt() else INVALID.toInt()

    fun decodeInto(text: CharSequence, out: ByteArray): Base32Exception? {
        val want = encodedLength(out.size)
        var acc = 0
        var bits = 0
        var symbols = 0
        var o = 0
        for (i in 0 until text.length) {
            val c = text[i]
            val v = symbolValue(c)
            if (v == SKIP.toInt()) continue
            if (v < 0) return Base32Exception(Base32Error.SYMBOL, c)
            if (++symbols > want) return Base32Exception(Base32Error.LENGTH)
            acc = (acc shl 5) or v
            bits += 5
            if (bits >= 8) {
                bits -= 8
                out[o++] = (acc ushr bits).toByte()
            }
        }
        if (symbols != want) return Base32Exception(Base32Error.LENGTH)
        if (acc and ((1 shl bits) - 1) != 0) return Base32Exception(Base32Error.PADDING)
        return null
    }

    fun decode(text: CharSequence, byteCount: Int): ByteArray {
        val out = ByteArray(byteCount)
        decodeInto(text, out)?.let { throw it }
        return out
    }
}
