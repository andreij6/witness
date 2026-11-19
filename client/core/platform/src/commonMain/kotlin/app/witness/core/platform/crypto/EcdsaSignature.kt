package app.witness.core.platform.crypto

object EcdsaSignature {
    const val COMPONENT = 32
    const val RAW_SIZE = 2 * COMPONENT

    private val ORDER = hex("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551")
    private val HALF_ORDER = hex("7fffffff800000007fffffffffffffffde737d56d38bcf4279dce5617e3192a8")

    fun derToRaw(der: ByteArray): ByteArray {
        val out = ByteArray(RAW_SIZE)
        var at = 0
        require(der.size >= 8 && der[at++] == SEQUENCE) { "not a DER sequence" }
        val seqLen = der[at++].toInt() and 0xff
        require(seqLen < 0x80 && seqLen == der.size - 2) { "bad sequence length" }
        at = readInteger(der, at, out, 0)
        at = readInteger(der, at, out, COMPONENT)
        require(at == der.size) { "trailing bytes" }
        normalizeLowS(out)
        return out
    }

    fun rawToDer(raw: ByteArray): ByteArray {
        require(raw.size == RAW_SIZE) { "raw signature must be $RAW_SIZE bytes" }
        val r = minimalInteger(raw, 0)
        val s = minimalInteger(raw, COMPONENT)
        val body = 2 + r.size + 2 + s.size
        val out = ByteArray(2 + body)
        out[0] = SEQUENCE
        out[1] = body.toByte()
        var at = 2
        for (part in arrayOf(r, s)) {
            out[at++] = INTEGER
            out[at++] = part.size.toByte()
            part.copyInto(out, at)
            at += part.size
        }
        return out
    }

    fun isLowS(raw: ByteArray): Boolean = compare(raw, COMPONENT, HALF_ORDER) <= 0

    private fun readInteger(der: ByteArray, start: Int, out: ByteArray, offset: Int): Int {
        var at = start
        require(at + 2 <= der.size && der[at++] == INTEGER) { "expected INTEGER" }
        val len = der[at++].toInt() and 0xff
        require(len in 1..COMPONENT + 1 && at + len <= der.size) { "bad INTEGER length" }
        require(der[at].toInt() and 0x80 == 0) { "negative INTEGER" }
        if (len > 1) require(der[at] != 0.toByte() || der[at + 1].toInt() and 0x80 != 0) { "non-minimal INTEGER" }
        var from = at
        var n = len
        if (n == COMPONENT + 1) {
            require(der[from] == 0.toByte()) { "INTEGER too large" }
            from++
            n--
        }
        der.copyInto(out, offset + COMPONENT - n, from, from + n)
        require(!isZero(out, offset) && compare(out, offset, ORDER) < 0) { "INTEGER out of range" }
        return at + len
    }

    private fun minimalInteger(raw: ByteArray, offset: Int): ByteArray {
        var from = offset
        val end = offset + COMPONENT
        while (from < end - 1 && raw[from] == 0.toByte()) from++
        val pad = if (raw[from].toInt() and 0x80 != 0) 1 else 0
        val out = ByteArray(end - from + pad)
        raw.copyInto(out, pad, from, end)
        return out
    }

    private fun normalizeLowS(raw: ByteArray) {
        if (compare(raw, COMPONENT, HALF_ORDER) <= 0) return
        var borrow = 0
        for (i in COMPONENT - 1 downTo 0) {
            val d = (ORDER[i].toInt() and 0xff) - (raw[COMPONENT + i].toInt() and 0xff) - borrow
            borrow = if (d < 0) 1 else 0
            raw[COMPONENT + i] = (d and 0xff).toByte()
        }
    }

    private fun compare(a: ByteArray, offset: Int, b: ByteArray): Int {
        for (i in 0 until COMPONENT) {
            val c = (a[offset + i].toInt() and 0xff) - (b[i].toInt() and 0xff)
            if (c != 0) return c
        }
        return 0
    }

    private fun isZero(a: ByteArray, offset: Int): Boolean {
        for (i in offset until offset + COMPONENT) if (a[i] != 0.toByte()) return false
        return true
    }

    private fun hex(s: String) = ByteArray(s.length / 2) { s.substring(2 * it, 2 * it + 2).toInt(16).toByte() }

    private const val SEQUENCE: Byte = 0x30
    private const val INTEGER: Byte = 0x02
}
