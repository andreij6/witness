package app.witness.core.data.codec

class ByteWriter(capacity: Int = 64) {
    private var buf = ByteArray(capacity)
    private var size = 0

    fun byte(v: Int): ByteWriter {
        ensure(1)
        buf[size++] = v.toByte()
        return this
    }

    fun int(v: Int): ByteWriter {
        ensure(4)
        for (i in 0 until 4) buf[size++] = (v ushr (24 - 8 * i)).toByte()
        return this
    }

    fun long(v: Long): ByteWriter {
        ensure(8)
        for (i in 0 until 8) buf[size++] = (v ushr (56 - 8 * i)).toByte()
        return this
    }

    fun bytes(v: ByteArray): ByteWriter {
        int(v.size)
        ensure(v.size)
        v.copyInto(buf, size)
        size += v.size
        return this
    }

    fun string(v: String): ByteWriter = bytes(v.encodeToByteArray())

    fun nullableLong(v: Long?): ByteWriter = if (v == null) byte(0) else byte(1).long(v)

    fun toByteArray(): ByteArray = buf.copyOf(size)

    private fun ensure(n: Int) {
        if (size + n <= buf.size) return
        var cap = buf.size.coerceAtLeast(16)
        while (cap < size + n) cap *= 2
        buf = buf.copyOf(cap)
    }
}

class ByteReader(private val buf: ByteArray) {
    private var at = 0

    val exhausted: Boolean get() = at == buf.size

    fun byte(): Int {
        need(1)
        return buf[at++].toInt() and 0xff
    }

    fun int(): Int {
        need(4)
        var v = 0
        repeat(4) { v = (v shl 8) or (buf[at++].toInt() and 0xff) }
        return v
    }

    fun long(): Long {
        need(8)
        var v = 0L
        repeat(8) { v = (v shl 8) or (buf[at++].toLong() and 0xff) }
        return v
    }

    fun bytes(max: Int = MAX_FIELD): ByteArray {
        val n = int()
        require(n in 0..max) { "field length $n out of range" }
        need(n)
        return buf.copyOfRange(at, at + n).also { at += n }
    }

    fun string(max: Int = MAX_FIELD): String = bytes(max).decodeToString(throwOnInvalidSequence = true)

    fun nullableLong(): Long? = when (byte()) {
        0 -> null
        1 -> long()
        else -> throw IllegalArgumentException("bad optional tag")
    }

    private fun need(n: Int) = require(n <= buf.size - at) { "truncated input" }

    companion object {
        const val MAX_FIELD = 1 shl 20
    }
}
