package app.witness.core.designsystem.qr

import com.google.zxing.DecodeHintType
import com.google.zxing.EncodeHintType
import com.google.zxing.common.BitMatrix
import com.google.zxing.qrcode.decoder.Decoder
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import com.google.zxing.qrcode.encoder.Encoder
import kotlin.test.Test
import kotlin.test.assertEquals

class QrZxingInteropTest {
    private val samples = listOf(
        "0",
        "01234567",
        "HELLO WORLD",
        "HTTPS://WITNESS.APP/P/H4IL-7QK",
        "witness.app/p/H4IL-7QK",
        "https://case.witness.app/v/8F2K-QM7D#k=Zm9vYmFyYmF6cXV4",
        "The quick brown fox jumps over the lazy dog 1234567890",
        "9".repeat(300),
        "A1 ".repeat(120),
        "x".repeat(900),
    )

    private fun level(ecc: QrEcc) = when (ecc) {
        QrEcc.L -> ErrorCorrectionLevel.L
        QrEcc.M -> ErrorCorrectionLevel.M
        QrEcc.Q -> ErrorCorrectionLevel.Q
        QrEcc.H -> ErrorCorrectionLevel.H
    }

    private fun QrMatrix.toBitMatrix(): BitMatrix {
        val bits = BitMatrix(size)
        for (y in 0 until size) for (x in 0 until size) if (this[x, y]) bits.set(x, y)
        return bits
    }

    private fun decode(qr: QrMatrix): String =
        Decoder().decode(qr.toBitMatrix(), mapOf(DecodeHintType.CHARACTER_SET to "UTF-8")).text

    @Test
    fun matchesZxingBitForBitForEveryLevelAndMask() {
        for (text in samples) for (ecc in QrEcc.entries) for (mask in 0..7) {
            val ours = QrEncoder.encode(text, ecc, mask = mask)
            val theirs = Encoder.encode(text, level(ecc), mapOf(EncodeHintType.QR_VERSION to ours.version, EncodeHintType.QR_MASK_PATTERN to mask))
            assertEquals(theirs.version.versionNumber, ours.version, "$text $ecc $mask")
            val m = theirs.matrix
            assertEquals(m.width, ours.size)
            for (y in 0 until ours.size) for (x in 0 until ours.size) {
                assertEquals(m.get(x, y).toInt() == 1, ours[x, y], "module ($x,$y) of '$text' $ecc mask $mask")
            }
        }
    }

    @Test
    fun minimumVersionMatchesZxing() {
        for (text in samples) for (ecc in QrEcc.entries) {
            assertEquals(Encoder.encode(text, level(ecc)).version.versionNumber, QrEncoder.encode(text, ecc).version, "$text $ecc")
        }
    }

    @Test
    fun zxingDecodesEveryVersion() {
        for (version in 1..40) for (ecc in listOf(QrEcc.L, QrEcc.H)) {
            val text = "v$version-$ecc"
            val qr = QrEncoder.encode(text, ecc, minVersion = version)
            assertEquals(version, qr.version)
            assertEquals(text, decode(qr))
        }
    }

    @Test
    fun zxingDecodesFullCapacitySymbols() {
        val bytes = "w".repeat(2953)
        assertEquals(bytes, decode(QrEncoder.encode(bytes, QrEcc.L)))
        val digits = (0 until 7089).joinToString("") { (it % 10).toString() }
        assertEquals(digits, decode(QrEncoder.encode(digits, QrEcc.L)))
    }

    @Test
    fun zxingDecodesUtf8() {
        val text = "Witness ✓ — Zürich · 東京"
        assertEquals(text, decode(QrEncoder.encode(text, QrEcc.Q)))
    }

    @Test
    fun zxingDecodesAutoMaskedProofLinks() {
        repeat(64) {
            val text = "https://witness.app/p/${(it * 7919).toString(36).uppercase().padStart(7, '0')}"
            assertEquals(text, decode(QrEncoder.encode(text)))
        }
    }
}
