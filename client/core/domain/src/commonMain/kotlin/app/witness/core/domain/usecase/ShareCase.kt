package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseBundle
import app.witness.core.domain.port.CaseBundleCodec
import app.witness.core.domain.port.CaseBundleItem
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.port.UploadRequest
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.Entitlement
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry
import app.witness.core.model.Tier
import kotlin.io.encoding.Base64
import kotlinx.coroutines.flow.first

sealed interface ShareOutcome {
    data class Shared(val share: CaseShare) : ShareOutcome

    data class TooLarge(val bytes: Long, val limit: Long) : ShareOutcome

    data object NothingToShare : ShareOutcome
}

class ShareCase(
    private val cases: CaseStore,
    private val captures: CaptureStore,
    private val codec: CaseBundleCodec,
    private val crypto: Crypto,
    private val media: MediaGateway,
    private val registry: RegistryGateway,
    private val entitlements: EntitlementSource,
    private val random: RandomSource,
    private val clock: Clock,
    private val links: Links = Links(),
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend operator fun invoke(
        caseId: LocalCaseId,
        items: List<RecordId>,
        expiry: ShareExpiry,
        includeSummary: Boolean = true,
    ): ShareOutcome {
        val case = cases.get(caseId) ?: throw CaseException("unknown case")
        val bundleItems = items.distinct().mapNotNull { id ->
            val capture = captures.get(id)?.takeIf { it.caseId == caseId && it.isVerified } ?: return@mapNotNull null
            val jpeg = captures.readImage(id) ?: return@mapNotNull null
            CaseBundleItem(capture.recordId, jpeg, null)
        }
        if (bundleItems.isEmpty()) return ShareOutcome.NothingToShare
        val plain = codec.encode(CaseBundle(case.title, case.summary.takeIf { includeSummary }, bundleItems))
        val now = clock.nowEpochMillis()
        val count = chunkCount(plain.size)
        val wireBytes = plain.size.toLong() + count.toLong() * CHUNK_OVERHEAD
        val limit = minOf(limitFor(entitlements.entitlement().first(), now), media.maxCaseBytes)
        if (wireBytes > limit) return ShareOutcome.TooLarge(wireBytes, limit)

        val key = random.nextBytes(Crypto.KEY_SIZE)
        val id = CaseId.of(pickShard(), random.nextBytes(CaseId.RANDOM_SIZE))
        val bundleHash = crypto.sha256(plain)
        val expiresAt = expiry.expiresAt(now)
        val ticket = media.begin(UploadRequest(id, count, wireBytes, ByteString.of(bundleHash), expiresAt))
        val idBytes = id.toBytes()
        for (ix in 0 until count) {
            val offset = ix * PLAIN_CHUNK
            val length = minOf(PLAIN_CHUNK, plain.size - offset)
            val nonce = random.nextBytes(Crypto.NONCE_SIZE)
            val sealed = crypto.aesGcmSeal(key, nonce, aad(idBytes, ix, count, bundleHash), plain, offset, length)
            media.putChunk(ticket, ix, nonce + sealed)
        }
        media.commit(ticket)
        val share = CaseShare(id, link(id, key, bundleHash), now, media.caseExpiry(id) ?: expiresAt, bundleItems.size)
        cases.update(case.copy(shares = case.shares + share, updatedAtMs = now))
        telemetry.event(AnalyticsEvent.CASE_SHARED, mapOf("items" to bundleItems.size.toLong(), "chunks" to count.toLong(), "expiry" to expiry.ordinal.toLong()))
        return ShareOutcome.Shared(share)
    }

    fun link(id: CaseId, key: ByteArray, bundleHash: ByteArray? = null): String =
        links.caseBase + id.text + "#k=" + B64URL.encode(key) + (bundleHash?.let { "&h=" + B64URL.encode(it) } ?: "")

    private suspend fun pickShard(): Int {
        val shards = registry.shards().media
        if (shards.isEmpty()) return 0
        val r = random.nextBytes(2)
        val v = ((r[0].toInt() and 0xff) shl 8) or (r[1].toInt() and 0xff)
        return shards[v % shards.size].id
    }

    companion object {
        val DOMAIN: ByteArray = "witness/case/v1".encodeToByteArray()
        const val CHUNK_OVERHEAD: Int = Crypto.NONCE_SIZE + Crypto.TAG_SIZE
        const val PLAIN_CHUNK: Int = MediaGateway.MAX_CHUNK_BYTES - CHUNK_OVERHEAD
        private val B64URL = Base64.UrlSafe.withPadding(Base64.PaddingOption.ABSENT_OPTIONAL)

        fun chunkCount(plainBytes: Int): Int = maxOf(1, (plainBytes + PLAIN_CHUNK - 1) / PLAIN_CHUNK)

        fun limitFor(entitlement: Entitlement, nowMs: Long): Long =
            if (entitlement.effectiveTier(nowMs) == Tier.PLUS) minOf(entitlement.quotaBytes, Entitlement.PLUS_CASE_BYTES) else Entitlement.FREE_CASE_BYTES

        fun aad(caseId: ByteArray, ix: Int, count: Int, bundleHash: ByteArray): ByteArray {
            val out = ByteArray(DOMAIN.size + caseId.size + 8 + bundleHash.size)
            DOMAIN.copyInto(out, 0)
            caseId.copyInto(out, DOMAIN.size)
            var p = DOMAIN.size + caseId.size
            p = putU32(out, p, ix)
            p = putU32(out, p, count)
            bundleHash.copyInto(out, p)
            return out
        }

        fun decodeKey(fragment: String): ByteArray? =
            try {
                val k = fragment.removePrefix("#").split('&').firstOrNull { it.startsWith("k=") } ?: return null
                B64URL.decode(k.removePrefix("k=")).takeIf { it.size == Crypto.KEY_SIZE }
            } catch (_: IllegalArgumentException) {
                null
            }

        private fun putU32(out: ByteArray, at: Int, v: Int): Int {
            out[at] = (v ushr 24).toByte()
            out[at + 1] = (v ushr 16).toByte()
            out[at + 2] = (v ushr 8).toByte()
            out[at + 3] = v.toByte()
            return at + 4
        }
    }
}
