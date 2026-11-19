package app.witness.feature.share

object Crc32 {
    private val TABLE = IntArray(256) { n ->
        var c = n
        repeat(8) { c = if (c and 1 != 0) (c ushr 1) xor 0xEDB88320.toInt() else c ushr 1 }
        c
    }

    fun update(crc: Int, data: ByteArray, offset: Int = 0, length: Int = data.size - offset): Int {
        require(offset >= 0 && length >= 0 && offset + length <= data.size) { "range out of bounds" }
        var c = crc.inv()
        for (i in offset until offset + length) c = TABLE[(c xor data[i].toInt()) and 0xff] xor (c ushr 8)
        return c.inv()
    }

    fun of(data: ByteArray): Int = update(0, data)
}

fun interface ByteSink {
    suspend fun write(bytes: ByteArray, offset: Int, length: Int)
}

class ZipWriter(private val sink: ByteSink, private val dosTime: Int = DOS_EPOCH_TIME, private val dosDate: Int = DOS_EPOCH_DATE) {
    private class Entry(val name: ByteArray, val crc: Int, val size: Long, val offset: Long)

    private val entries = ArrayList<Entry>()
    private val names = HashSet<String>()
    private var position = 0L
    private var finished = false

    val bytesWritten: Long get() = position

    suspend fun add(name: String, data: ByteArray) {
        check(!finished) { "zip already finished" }
        require(isSafeName(name)) { "unsafe entry name: $name" }
        require(names.add(name)) { "duplicate entry: $name" }
        require(entries.size < MAX_ENTRIES) { "too many entries" }
        val encoded = name.encodeToByteArray()
        val offset = position
        require(offset + LOCAL_HEADER + encoded.size + data.size < LIMIT) { "archive too large" }
        val crc = Crc32.of(data)
        val header = ByteArray(LOCAL_HEADER + encoded.size)
        var p = putInt(header, 0, LOCAL_SIG)
        p = putShort(header, p, VERSION)
        p = putShort(header, p, FLAG_UTF8)
        p = putShort(header, p, METHOD_STORE)
        p = putShort(header, p, dosTime)
        p = putShort(header, p, dosDate)
        p = putInt(header, p, crc)
        p = putInt(header, p, data.size)
        p = putInt(header, p, data.size)
        p = putShort(header, p, encoded.size)
        p = putShort(header, p, 0)
        encoded.copyInto(header, p)
        emit(header)
        emit(data)
        entries += Entry(encoded, crc, data.size.toLong(), offset)
    }

    suspend fun finish() {
        check(!finished) { "zip already finished" }
        finished = true
        val start = position
        for (e in entries) {
            val header = ByteArray(CENTRAL_HEADER + e.name.size)
            var p = putInt(header, 0, CENTRAL_SIG)
            p = putShort(header, p, VERSION)
            p = putShort(header, p, VERSION)
            p = putShort(header, p, FLAG_UTF8)
            p = putShort(header, p, METHOD_STORE)
            p = putShort(header, p, dosTime)
            p = putShort(header, p, dosDate)
            p = putInt(header, p, e.crc)
            p = putInt(header, p, e.size.toInt())
            p = putInt(header, p, e.size.toInt())
            p = putShort(header, p, e.name.size)
            p = putShort(header, p, 0)
            p = putShort(header, p, 0)
            p = putShort(header, p, 0)
            p = putShort(header, p, 0)
            p = putInt(header, p, 0)
            p = putInt(header, p, e.offset.toInt())
            e.name.copyInto(header, p)
            emit(header)
        }
        val size = position - start
        require(position + END_RECORD < LIMIT) { "archive too large" }
        val end = ByteArray(END_RECORD)
        var p = putInt(end, 0, END_SIG)
        p = putShort(end, p, 0)
        p = putShort(end, p, 0)
        p = putShort(end, p, entries.size)
        p = putShort(end, p, entries.size)
        p = putInt(end, p, size.toInt())
        p = putInt(end, p, start.toInt())
        putShort(end, p, 0)
        emit(end)
    }

    private suspend fun emit(bytes: ByteArray) {
        sink.write(bytes, 0, bytes.size)
        position += bytes.size
    }

    companion object {
        const val LOCAL_SIG: Int = 0x04034b50
        const val CENTRAL_SIG: Int = 0x02014b50
        const val END_SIG: Int = 0x06054b50
        const val LOCAL_HEADER: Int = 30
        const val CENTRAL_HEADER: Int = 46
        const val END_RECORD: Int = 22
        const val VERSION: Int = 20
        const val FLAG_UTF8: Int = 0x0800
        const val METHOD_STORE: Int = 0
        const val MAX_ENTRIES: Int = 0xFFFF
        const val LIMIT: Long = 0xFFFF_FFFFL
        const val DOS_EPOCH_TIME: Int = 0
        const val DOS_EPOCH_DATE: Int = (0 shl 9) or (1 shl 5) or 1

        fun isSafeName(name: String): Boolean =
            name.isNotEmpty() && name.length <= 255 && !name.startsWith('/') && '\\' !in name && name.split('/').none { it.isEmpty() || it == "." || it == ".." } &&
                name.none { it.code < 0x20 }

        fun dosDateTime(year: Long, month: Int, day: Int, hour: Int, minute: Int, second: Int): Pair<Int, Int> {
            if (year < 1980) return DOS_EPOCH_TIME to DOS_EPOCH_DATE
            val y = (year - 1980).coerceAtMost(127).toInt()
            return ((hour shl 11) or (minute shl 5) or (second / 2)) to ((y shl 9) or (month shl 5) or day)
        }

        private fun putShort(out: ByteArray, at: Int, v: Int): Int {
            out[at] = v.toByte()
            out[at + 1] = (v ushr 8).toByte()
            return at + 2
        }

        private fun putInt(out: ByteArray, at: Int, v: Int): Int {
            out[at] = v.toByte()
            out[at + 1] = (v ushr 8).toByte()
            out[at + 2] = (v ushr 16).toByte()
            out[at + 3] = (v ushr 24).toByte()
            return at + 4
        }
    }
}
