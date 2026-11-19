package app.witness.feature.settings

import kotlin.jvm.JvmInline

@JvmInline
value class Principal private constructor(val text: String) {
    val isSelfAuthenticating: Boolean get() = decodeBody(text)?.let(::selfAuthenticating) == true

    val short: String get() = if (text.length <= SHORT_KEEP * 2 + 1) text else text.take(SHORT_KEEP) + "…" + text.takeLast(SHORT_KEEP)

    override fun toString(): String = text

    companion object {
        const val MAX_BYTES: Int = 29
        const val MAX_TEXT: Int = 63
        private const val SELF_AUTHENTICATING_TAG: Byte = 0x02
        private const val SHORT_KEEP = 5
        private const val CRC_BYTES = 4
        private const val GROUP = 5
        private const val ALPHABET = "abcdefghijklmnopqrstuvwxyz234567"

        fun parse(text: String): Principal? = if (decodeBody(text) != null) Principal(text) else null

        fun parseSelfAuthenticating(text: String): Principal? = parse(text)?.takeIf { it.isSelfAuthenticating }

        fun fromBytes(bytes: ByteArray): Principal {
            require(bytes.size <= MAX_BYTES) { "principal longer than $MAX_BYTES bytes" }
            return Principal(encode(bytes))
        }

        fun encode(bytes: ByteArray): String {
            val crc = Crc32.of(bytes)
            val raw = ByteArray(CRC_BYTES + bytes.size)
            for (i in 0 until CRC_BYTES) raw[i] = (crc ushr (24 - 8 * i)).toByte()
            bytes.copyInto(raw, CRC_BYTES)
            val chars = (raw.size * 8 + 4) / 5
            val out = StringBuilder(chars + chars / GROUP)
            var buffer = 0
            var bits = 0
            var written = 0
            fun emit(index: Int) {
                if (written > 0 && written % GROUP == 0) out.append('-')
                out.append(ALPHABET[index])
                written++
            }
            for (b in raw) {
                buffer = (buffer shl 8) or (b.toInt() and 0xFF)
                bits += 8
                while (bits >= 5) {
                    bits -= 5
                    emit((buffer ushr bits) and 31)
                }
                buffer = buffer and ((1 shl bits) - 1)
            }
            if (bits > 0) emit((buffer shl (5 - bits)) and 31)
            return out.toString()
        }

        fun decodeBody(text: String): ByteArray? {
            if (text.isEmpty() || text.length > MAX_TEXT) return null
            val raw = ByteArray(text.length * 5 / 8)
            var buffer = 0
            var bits = 0
            var n = 0
            for (c in text) {
                if (c == '-') continue
                val v = when (c) {
                    in 'a'..'z' -> c - 'a'
                    in '2'..'7' -> c - '2' + 26
                    else -> return null
                }
                buffer = (buffer shl 5) or v
                bits += 5
                if (bits >= 8) {
                    bits -= 8
                    if (n == raw.size) return null
                    raw[n++] = (buffer ushr bits).toByte()
                    buffer = buffer and ((1 shl bits) - 1)
                }
            }
            if (n < CRC_BYTES || n - CRC_BYTES > MAX_BYTES) return null
            val body = raw.copyOfRange(CRC_BYTES, n)
            return body.takeIf { encode(it) == text }
        }

        private fun selfAuthenticating(body: ByteArray): Boolean = body.size == MAX_BYTES && body.last() == SELF_AUTHENTICATING_TAG
    }
}

internal object Crc32 {
    private val table = IntArray(256) { n ->
        var c = n
        repeat(8) { c = if (c and 1 != 0) (c ushr 1) xor 0xEDB88320.toInt() else c ushr 1 }
        c
    }

    fun of(bytes: ByteArray): Int {
        var crc = -1
        for (b in bytes) crc = table[(crc xor b.toInt()) and 0xFF] xor (crc ushr 8)
        return crc.inv()
    }
}
