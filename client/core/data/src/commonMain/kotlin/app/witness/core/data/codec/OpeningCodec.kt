package app.witness.core.data.codec

import app.witness.core.model.ByteString
import app.witness.core.model.LocationOpening

object OpeningCodec {
    const val VERSION = 1
    const val SIZE = 1 + LocationOpening.SALT_SIZE + 12

    fun encode(opening: LocationOpening): ByteArray {
        require(opening.salt.size == LocationOpening.SALT_SIZE) { "salt must be ${LocationOpening.SALT_SIZE} bytes" }
        require(opening.inRange) { "opening out of range" }
        val out = ByteArray(SIZE)
        out[0] = VERSION.toByte()
        opening.salt.copyInto(out, 1)
        val w = ByteWriter(12).int(opening.latE7).int(opening.lonE7).int(opening.accuracyCm).toByteArray()
        w.copyInto(out, 1 + LocationOpening.SALT_SIZE)
        return out
    }

    fun decode(bytes: ByteArray): LocationOpening {
        require(bytes.size == SIZE) { "opening must be $SIZE bytes" }
        require(bytes[0].toInt() == VERSION) { "unsupported opening version" }
        val r = ByteReader(bytes.copyOfRange(1 + LocationOpening.SALT_SIZE, SIZE))
        val o = LocationOpening(ByteString.of(bytes.copyOfRange(1, 1 + LocationOpening.SALT_SIZE)), r.int(), r.int(), r.int())
        require(o.inRange) { "opening out of range" }
        return o
    }
}
