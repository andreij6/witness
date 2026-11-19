package app.witness.core.data.codec

import app.witness.core.model.BlurClass
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.ImageFormat
import app.witness.core.model.Rect

object EditCodec {
    const val VERSION = 1

    fun encode(edits: List<Edit>): ByteArray {
        require(edits.size <= Edit.MAX_EDITS) { "too many edits" }
        val w = ByteWriter(4 + edits.size * 20).byte(VERSION).byte(edits.size)
        for (e in edits) {
            w.byte(e.kind.ordinal)
            when (e) {
                is Edit.Resize -> w.int(e.w).int(e.h).int(e.filter)
                is Edit.Recompress -> w.byte(e.format.ordinal).int(e.quality)
                is Edit.Orient -> w.int(e.orientation)
                is Edit.Crop -> w.rect(e.rect)
                is Edit.Rotate -> w.int(e.degE2)
                is Edit.Tone -> w.int(e.exposureE2).int(e.contrastE2).int(e.saturationE2)
                is Edit.Filter -> w.int(e.id).int(e.strengthE2)
                is Edit.Blur -> w.rect(e.rect).byte(e.blurClass.ordinal).int(e.sigmaE2)
            }
        }
        return w.toByteArray()
    }

    fun decode(bytes: ByteArray): List<Edit> {
        val r = ByteReader(bytes)
        require(r.byte() == VERSION) { "unsupported edit version" }
        val n = r.byte()
        require(n <= Edit.MAX_EDITS) { "too many edits" }
        val out = ArrayList<Edit>(n)
        repeat(n) {
            out += when (EditKind.entries.getOrNull(r.byte()) ?: throw IllegalArgumentException("unknown edit kind")) {
                EditKind.RESIZE -> Edit.Resize(r.int(), r.int(), r.int())
                EditKind.RECOMPRESS -> Edit.Recompress(ImageFormat.entries.getOrNull(r.byte()) ?: throw IllegalArgumentException("format"), r.int())
                EditKind.ORIENT -> Edit.Orient(r.int())
                EditKind.CROP -> Edit.Crop(r.rect())
                EditKind.ROTATE -> Edit.Rotate(r.int())
                EditKind.TONE -> Edit.Tone(r.int(), r.int(), r.int())
                EditKind.FILTER -> Edit.Filter(r.int(), r.int())
                EditKind.BLUR -> Edit.Blur(r.rect(), BlurClass.entries.getOrNull(r.byte()) ?: throw IllegalArgumentException("blur class"), r.int())
            }
        }
        require(r.exhausted) { "trailing bytes" }
        return out
    }

    private fun ByteWriter.rect(v: Rect) = int(v.x).int(v.y).int(v.w).int(v.h)

    private fun ByteReader.rect() = Rect(int(), int(), int(), int())
}
