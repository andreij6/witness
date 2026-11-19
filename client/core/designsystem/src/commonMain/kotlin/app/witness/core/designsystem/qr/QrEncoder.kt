package app.witness.core.designsystem.qr

object QrEncoder {
    private const val Alphanumerics = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"
    private const val Pad0 = 0xEC
    private const val Pad1 = 0x11

    fun encode(text: String, ecc: QrEcc = QrEcc.M, minVersion: Int = QrTables.MinVersion, mask: Int = -1, boostEcc: Boolean = false): QrMatrix {
        val mode = modeFor(text)
        return if (mode == QrMode.Byte) {
            encodeSegment(QrMode.Byte, text.encodeToByteArray(), null, ecc, minVersion, mask, boostEcc)
        } else {
            encodeSegment(mode, null, text, ecc, minVersion, mask, boostEcc)
        }
    }

    fun encodeBytes(data: ByteArray, ecc: QrEcc = QrEcc.M, minVersion: Int = QrTables.MinVersion, mask: Int = -1, boostEcc: Boolean = false): QrMatrix =
        encodeSegment(QrMode.Byte, data, null, ecc, minVersion, mask, boostEcc)

    fun modeFor(text: String): QrMode = when {
        text.isEmpty() -> QrMode.Byte
        text.all { it in '0'..'9' } -> QrMode.Numeric
        text.all { Alphanumerics.indexOf(it) >= 0 } -> QrMode.Alphanumeric
        else -> QrMode.Byte
    }

    fun payloadBits(mode: QrMode, count: Int): Int = when (mode) {
        QrMode.Numeric -> count / 3 * 10 + intArrayOf(0, 4, 7)[count % 3]
        QrMode.Alphanumeric -> count / 2 * 11 + count % 2 * 6
        QrMode.Byte -> count * 8
    }

    fun minimumVersion(mode: QrMode, count: Int, ecc: QrEcc, minVersion: Int = QrTables.MinVersion): Int? {
        val payload = payloadBits(mode, count)
        for (version in minVersion..QrTables.MaxVersion) {
            val countBits = mode.characterCountBits(version)
            if (count >= 1 shl countBits) continue
            if (4 + countBits + payload <= QrTables.dataCodewords(version, ecc) * 8) return version
        }
        return null
    }

    private fun encodeSegment(mode: QrMode, bytes: ByteArray?, chars: String?, ecc: QrEcc, minVersion: Int, mask: Int, boostEcc: Boolean): QrMatrix {
        require(minVersion in QrTables.MinVersion..QrTables.MaxVersion) { "minVersion out of range: $minVersion" }
        require(mask in -1..7) { "mask out of range: $mask" }
        val count = bytes?.size ?: chars!!.length
        val version = minimumVersion(mode, count, ecc, minVersion) ?: throw IllegalArgumentException("Data too long for a QR code: $count units in $mode")
        var level = ecc
        if (boostEcc) {
            for (candidate in QrEcc.entries) {
                if (candidate.strength > level.strength && minimumVersion(mode, count, candidate, version) == version) level = candidate
            }
        }
        val capacity = QrTables.dataCodewords(version, level)
        val bits = BitWriter(capacity)
        bits.write(mode.indicator, 4)
        bits.write(count, mode.characterCountBits(version))
        when (mode) {
            QrMode.Numeric -> writeNumeric(bits, chars!!)
            QrMode.Alphanumeric -> writeAlphanumeric(bits, chars!!)
            QrMode.Byte -> for (b in bytes!!) bits.write(b.toInt() and 0xFF, 8)
        }
        bits.finish(capacity)
        val codewords = interleave(bits.bytes, version, level)
        return QrMatrixBuilder(version, level).build(codewords, mask, mode)
    }

    private val QrEcc.strength: Int get() = when (this) {
        QrEcc.L -> 0
        QrEcc.M -> 1
        QrEcc.Q -> 2
        QrEcc.H -> 3
    }

    private fun writeNumeric(bits: BitWriter, text: String) {
        var i = 0
        while (i < text.length) {
            val n = minOf(3, text.length - i)
            var value = 0
            for (k in i until i + n) value = value * 10 + (text[k] - '0')
            bits.write(value, n * 3 + 1)
            i += n
        }
    }

    private fun writeAlphanumeric(bits: BitWriter, text: String) {
        var i = 0
        while (i + 1 < text.length) {
            bits.write(Alphanumerics.indexOf(text[i]) * 45 + Alphanumerics.indexOf(text[i + 1]), 11)
            i += 2
        }
        if (i < text.length) bits.write(Alphanumerics.indexOf(text[i]), 6)
    }

    internal fun interleave(data: ByteArray, version: Int, ecc: QrEcc): ByteArray {
        val blocks = QrTables.blocks(version, ecc)
        val eccLen = QrTables.eccPerBlock(version, ecc)
        val raw = QrTables.rawDataModules(version) / 8
        val shortBlocks = blocks - raw % blocks
        val shortLen = raw / blocks
        val shortData = shortLen - eccLen
        val divisor = ReedSolomon.divisor(eccLen)
        val eccBytes = ByteArray(blocks * eccLen)
        val starts = IntArray(blocks)
        var offset = 0
        for (b in 0 until blocks) {
            val len = shortData + if (b < shortBlocks) 0 else 1
            starts[b] = offset
            ReedSolomon.remainder(data, offset, len, divisor, eccBytes, b * eccLen)
            offset += len
        }
        val out = ByteArray(raw)
        var k = 0
        for (i in 0..shortData) {
            for (b in 0 until blocks) {
                if (i == shortData && b < shortBlocks) continue
                out[k++] = data[starts[b] + i]
            }
        }
        for (i in 0 until eccLen) for (b in 0 until blocks) out[k++] = eccBytes[b * eccLen + i]
        return out
    }

    private class BitWriter(capacityBytes: Int) {
        val bytes = ByteArray(capacityBytes)
        var length = 0
            private set

        fun write(value: Int, count: Int) {
            for (i in count - 1 downTo 0) {
                if (value ushr i and 1 != 0) {
                    val index = length ushr 3
                    bytes[index] = (bytes[index].toInt() or (0x80 ushr (length and 7))).toByte()
                }
                length++
            }
        }

        fun finish(capacityBytes: Int) {
            val capacityBits = capacityBytes * 8
            length += minOf(4, capacityBits - length)
            length = (length + 7) and 7.inv()
            var pad = Pad0
            for (i in length ushr 3 until capacityBytes) {
                bytes[i] = pad.toByte()
                pad = pad xor Pad0 xor Pad1
            }
            length = capacityBits
        }
    }
}
