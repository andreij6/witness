package app.witness.core.domain.fake

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.CaseBundle
import app.witness.core.domain.port.CaseBundleCodec
import app.witness.core.domain.port.CaseBundleItem
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.ByteString
import app.witness.core.model.RecordId
import kotlin.random.Random

class FakeClock(var nowMs: Long = 1_763_510_400_000) : Clock {
    override fun nowEpochMillis(): Long = nowMs

    fun advance(ms: Long) {
        nowMs += ms
    }
}

class SeededRandom(seed: Long = 1) : RandomSource {
    private val random = Random(seed)

    override fun nextBytes(size: Int): ByteArray = random.nextBytes(size)
}

class FakeCrypto : Crypto {
    override suspend fun sha256(data: ByteArray, offset: Int, length: Int): ByteArray = SoftwareSha256.digest(data, offset, length)

    override suspend fun aesGcmSeal(key: ByteArray, nonce: ByteArray, aad: ByteArray, plaintext: ByteArray, offset: Int, length: Int): ByteArray {
        require(key.size == Crypto.KEY_SIZE && nonce.size == Crypto.NONCE_SIZE) { "bad key or nonce" }
        val out = ByteArray(length + Crypto.TAG_SIZE)
        xorStream(key, nonce, plaintext, offset, length, out)
        tag(key, nonce, aad, out, length).copyInto(out, length)
        return out
    }

    fun open(key: ByteArray, nonce: ByteArray, aad: ByteArray, sealed: ByteArray): ByteArray? {
        if (sealed.size < Crypto.TAG_SIZE) return null
        val length = sealed.size - Crypto.TAG_SIZE
        val expected = tag(key, nonce, aad, sealed, length)
        if (!expected.contentEquals(sealed.copyOfRange(length, sealed.size))) return null
        val out = ByteArray(length)
        xorStream(key, nonce, sealed, 0, length, out)
        return out
    }

    private fun xorStream(key: ByteArray, nonce: ByteArray, input: ByteArray, offset: Int, length: Int, out: ByteArray) {
        var block = ByteArray(0)
        for (i in 0 until length) {
            if (i % 32 == 0) block = SoftwareSha256.digest(key + nonce + counter(i / 32))
            out[i] = (input[offset + i].toInt() xor block[i % 32].toInt()).toByte()
        }
    }

    private fun tag(key: ByteArray, nonce: ByteArray, aad: ByteArray, ct: ByteArray, length: Int): ByteArray =
        SoftwareSha256.digest(key + nonce + counter(aad.size) + aad + ct.copyOf(length)).copyOf(Crypto.TAG_SIZE)

    private fun counter(v: Int) = byteArrayOf((v ushr 24).toByte(), (v ushr 16).toByte(), (v ushr 8).toByte(), v.toByte())
}

class FakeTelemetry : Telemetry {
    var collectionEnabled: Boolean? = null
        private set
    val events = ArrayList<Pair<AnalyticsEvent, Map<String, Long>>>()
    val breadcrumbs = ArrayList<String>()
    val errors = ArrayList<Throwable>()

    override fun setCollectionEnabled(enabled: Boolean) {
        collectionEnabled = enabled
    }

    override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) {
        events += event to metrics
    }

    override fun breadcrumb(message: String) {
        breadcrumbs += message
    }

    override fun nonFatal(error: Throwable) {
        errors += error
    }
}

class FakeAnchorScheduler : AnchorScheduler {
    var requests: Int = 0
        private set

    override fun requestAnchoring() {
        requests++
    }
}

class FakeCaseBundleCodec : CaseBundleCodec {
    override fun encode(bundle: CaseBundle): ByteArray {
        val parts = ArrayList<ByteArray>()
        parts += bundle.title.encodeToByteArray()
        parts += bundle.summary.orEmpty().encodeToByteArray()
        parts += byteArrayOf(bundle.items.size.toByte())
        for (item in bundle.items) {
            parts += item.recordId.toBytes()
            parts += item.jpeg.toByteArray()
        }
        val out = ByteArray(parts.sumOf { it.size + 4 })
        var p = 0
        for (part in parts) {
            for (s in intArrayOf(24, 16, 8, 0)) out[p++] = (part.size ushr s).toByte()
            part.copyInto(out, p)
            p += part.size
        }
        return out
    }

    fun decode(bytes: ByteArray): CaseBundle {
        var p = 0
        fun take(): ByteArray {
            var n = 0
            repeat(4) { n = (n shl 8) or (bytes[p++].toInt() and 0xff) }
            return bytes.copyOfRange(p, p + n).also { p += n }
        }
        val title = take().decodeToString()
        val summary = take().decodeToString().ifEmpty { null }
        val count = take()[0].toInt()
        val items = List(count) { CaseBundleItem(RecordId.fromBytes(take()), ByteString.of(take()), null) }
        return CaseBundle(title, summary, items)
    }
}
