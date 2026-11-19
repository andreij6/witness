package app.witness.core.model

enum class Base32Error { LENGTH, SYMBOL, PADDING }

class IdFormatException(val error: Base32Error, message: String) : IllegalArgumentException(message)

object Crockford {
    const val ALPHABET: String = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

    private const val INVALID: Byte = -1
    private const val SKIP: Byte = -2

    private val decodeTable = ByteArray(128) { INVALID }.also { t ->
        ALPHABET.forEachIndexed { i, c ->
            t[c.code] = i.toByte()
            t[c.lowercaseChar().code] = i.toByte()
        }
        for (c in "IiLl") t[c.code] = 1
        for (c in "Oo") t[c.code] = 0
        t['-'.code] = SKIP
    }

    fun encodedLength(bytes: Int): Int = (bytes * 8 + 4) / 5

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
            acc = acc and ((1 shl bits) - 1)
        }
        if (bits > 0) out[o] = ALPHABET[(acc shl (5 - bits)) and 31]
        return out.concatToString()
    }

    fun decode(text: CharSequence, size: Int): ByteArray {
        val want = encodedLength(size)
        val out = ByteArray(size)
        var acc = 0
        var bits = 0
        var symbols = 0
        var o = 0
        for (c in text) {
            val v = if (c.code < 128) decodeTable[c.code] else INVALID
            if (v == SKIP) continue
            if (v == INVALID) throw IdFormatException(Base32Error.SYMBOL, "invalid base32 symbol '$c'")
            if (++symbols > want) throw IdFormatException(Base32Error.LENGTH, "too many base32 symbols")
            acc = (acc shl 5) or v.toInt()
            bits += 5
            if (bits >= 8) {
                bits -= 8
                out[o++] = (acc ushr bits).toByte()
            }
            acc = acc and ((1 shl bits) - 1)
        }
        if (symbols != want) throw IdFormatException(Base32Error.LENGTH, "expected $want base32 symbols, got $symbols")
        if (acc != 0) throw IdFormatException(Base32Error.PADDING, "non-zero base32 padding bits")
        return out
    }

    fun canonicalOrNull(text: CharSequence, size: Int): String? =
        try {
            encode(decode(text, size))
        } catch (_: IdFormatException) {
            null
        }

    fun group(plain: String, groupSize: Int = 4): String {
        if (plain.length <= groupSize) return plain
        val sb = StringBuilder(plain.length + (plain.length - 1) / groupSize)
        plain.forEachIndexed { i, c ->
            if (i > 0 && i % groupSize == 0) sb.append('-')
            sb.append(c)
        }
        return sb.toString()
    }
}
