package app.witness.core.domain.usecase

import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.ShardMap
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaseId
import app.witness.core.model.Entitlement
import app.witness.core.model.LocalCaseId
import app.witness.core.model.ShareExpiry
import app.witness.core.model.Tier
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ShareCaseTest {
    private suspend fun caseWith(g: TestGraph, n: Int, bytesEach: Int = 100): Pair<LocalCaseId, List<Capture>> {
        g.enrolledKey()
        val case = g.manageCases.create("Fender bender")
        val items = List(n) { i ->
            g.sealOne().also { g.captures.putImage(it.recordId, ByteString.of(ByteArray(bytesEach) { b -> (b + i).toByte() })) }
        }
        return case.id to items
    }

    @Test
    fun chunkMath() {
        assertEquals(MediaGateway.MAX_CHUNK_BYTES - 28, ShareCase.PLAIN_CHUNK)
        assertEquals(1, ShareCase.chunkCount(0))
        assertEquals(1, ShareCase.chunkCount(1))
        assertEquals(1, ShareCase.chunkCount(ShareCase.PLAIN_CHUNK))
        assertEquals(2, ShareCase.chunkCount(ShareCase.PLAIN_CHUNK + 1))
        assertEquals(16, ShareCase.chunkCount(16 * ShareCase.PLAIN_CHUNK))
    }

    @Test
    fun aadLayoutFollowsArchitecture() {
        val id = ByteArray(16) { it.toByte() }
        val hash = ByteArray(32) { 0x55 }
        val aad = ShareCase.aad(id, 0x01020304, 0x0a0b0c0d, hash)
        val domain = "witness/case/v1".encodeToByteArray()
        assertEquals(domain.size + 16 + 8 + 32, aad.size)
        assertContentEquals(domain, aad.copyOfRange(0, domain.size))
        assertContentEquals(id, aad.copyOfRange(domain.size, domain.size + 16))
        assertContentEquals(byteArrayOf(1, 2, 3, 4, 10, 11, 12, 13), aad.copyOfRange(domain.size + 16, domain.size + 24))
        assertContentEquals(hash, aad.copyOfRange(domain.size + 24, aad.size))
    }

    @Test
    fun limitsDependOnEntitlement() {
        assertEquals(Entitlement.FREE_CASE_BYTES, ShareCase.limitFor(Entitlement.FREE, 0))
        val plus = Entitlement(Tier.PLUS, 1L shl 34, 100)
        assertEquals(Entitlement.PLUS_CASE_BYTES, ShareCase.limitFor(plus, 99))
        assertEquals(Entitlement.FREE_CASE_BYTES, ShareCase.limitFor(plus, 100))
    }

    @Test
    fun caseLimitsMatchTheMediaCanister() {
        assertEquals(16L shl 20, Entitlement.FREE_CASE_BYTES)
        assertEquals(256L shl 20, Entitlement.PLUS_CASE_BYTES)
        assertEquals(Entitlement.PLUS_CASE_BYTES, ShareCase.limitFor(Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, null), 0))
        assertEquals(5_000_000L, ShareCase.limitFor(Entitlement(Tier.PLUS, 5_000_000L, null), 0))
        assertEquals(0L, ShareCase.limitFor(Entitlement(Tier.PLUS, 0L, null), 0))
        assertEquals(16, ShareCase.chunkCount((Entitlement.FREE_CASE_BYTES - 16L * ShareCase.CHUNK_OVERHEAD).toInt()))
        assertEquals(256, ShareCase.chunkCount((Entitlement.PLUS_CASE_BYTES - 256L * ShareCase.CHUNK_OVERHEAD).toInt()))
        assertEquals(17, ShareCase.chunkCount((Entitlement.FREE_CASE_BYTES - 16L * ShareCase.CHUNK_OVERHEAD).toInt() + 1))
    }

    @Test
    fun shareRecordsTheExpiryTheMediaCanisterReports() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 1)
        val chosen = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY)).share
        assertEquals(g.clock.nowMs + 86_400_000L, chosen.expiresAtMs)
        g.media.caseExpiryMs = g.clock.nowMs + 30L * 86_400_000
        val shared = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.NEVER)).share
        assertEquals(g.media.caseExpiryMs, shared.expiresAtMs)
        assertEquals(shared, g.cases.get(caseId)!!.shares.last())
        assertNull(g.media.caseExpiry(CaseId.of(0, ByteArray(CaseId.RANDOM_SIZE))))
    }

    @Test
    fun linkCarriesKeyInFragmentOnly() = runTest {
        val g = TestGraph()
        val id = CaseId.of(3, ByteArray(15) { 9 })
        val key = ByteArray(32) { (it * 7).toByte() }
        val link = g.share.link(id, key)
        assertTrue(link.startsWith("https://case.witness.app/v/${id.text}#k="))
        val fragment = link.substringAfter('#')
        assertTrue(fragment.startsWith("k="))
        assertTrue(fragment.removePrefix("k=").none { it == '=' || it == '+' || it == '/' })
        assertContentEquals(key, ShareCase.decodeKey(fragment))
        assertContentEquals(key, ShareCase.decodeKey("#$fragment"))
        assertNull(ShareCase.decodeKey("k=short"))
        assertNull(ShareCase.decodeKey("k=@@@"))
        assertNull(ShareCase.decodeKey("h=abc"))
        assertContentEquals(key, ShareCase.decodeKey("$fragment="))
        assertContentEquals(key, ShareCase.decodeKey("x=1&$fragment=&h=2"))
        assertNull(ShareCase.decodeKey("$fragment=="))
        assertNull(ShareCase.decodeKey("k="))
        assertNull(ShareCase.decodeKey(""))
    }

    @Test
    fun linkCarriesTheBundleHashTheWebNeedsToOpenTheCase() = runTest {
        val g = TestGraph()
        val id = CaseId.of(3, ByteArray(15) { 9 })
        val key = ByteArray(32) { (it * 7).toByte() }
        val hash = ByteArray(32) { (255 - it).toByte() }
        val link = g.share.link(id, key, hash)
        val fragment = link.substringAfter('#')
        val params = fragment.split('&').associate { it.substringBefore('=') to it.substringAfter('=') }
        assertEquals(setOf("k", "h"), params.keys)
        assertContentEquals(hash, kotlin.io.encoding.Base64.UrlSafe.withPadding(kotlin.io.encoding.Base64.PaddingOption.ABSENT).decode(params.getValue("h")))
        assertTrue(params.getValue("h").none { it == '=' || it == '+' || it == '/' })
        assertContentEquals(key, ShareCase.decodeKey(fragment))
        assertContentEquals(key, ShareCase.decodeKey("#h=${params.getValue("h")}&k=${params.getValue("k")}"))
    }

    @Test
    fun sharesEncryptedBundleThatRecipientCanOpen() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 3)
        val outcome = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.WEEK))
        val share = outcome.share
        assertEquals(3, share.itemCount)
        assertEquals(g.clock.nowMs + 7 * 86_400_000L, share.expiresAtMs)
        assertEquals(listOf(share), g.cases.get(caseId)!!.shares)

        val upload = g.media.uploads.values.single()
        assertTrue(upload.committed)
        assertEquals(share.caseId, upload.request.caseId)
        val key = assertNotNull(ShareCase.decodeKey(share.url.substringAfter('#')))
        assertTrue(share.url.endsWith("&h=" + kotlin.io.encoding.Base64.UrlSafe.withPadding(kotlin.io.encoding.Base64.PaddingOption.ABSENT).encode(upload.request.bundleHash.toByteArray())))
        val plain = ByteArray(0).let { acc ->
            var out = acc
            for (ix in 0 until upload.request.chunkCount) {
                val chunk = upload.chunks.getValue(ix)
                val nonce = chunk.copyOf(Crypto.NONCE_SIZE)
                val aad = ShareCase.aad(share.caseId.toBytes(), ix, upload.request.chunkCount, upload.request.bundleHash.toByteArray())
                out += assertNotNull(g.crypto.open(key, nonce, aad, chunk.copyOfRange(Crypto.NONCE_SIZE, chunk.size)))
            }
            out
        }
        assertEquals(upload.request.bundleHash, ByteString.of(SoftwareSha256.digest(plain)))
        val bundle = g.codec.decode(plain)
        assertEquals("Fender bender", bundle.title)
        assertEquals(items.map { it.recordId }, bundle.items.map { it.recordId })
        assertEquals(upload.request.totalBytes, upload.chunks.values.sumOf { it.size.toLong() })
    }

    @Test
    fun largeBundlesSplitIntoBoundedChunksBoundToTheirPosition() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 3, bytesEach = 900_000)
        val share = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY)).share
        val upload = g.media.uploads.values.single()
        assertEquals(3, upload.request.chunkCount)
        assertTrue(upload.chunks.values.all { it.size <= MediaGateway.MAX_CHUNK_BYTES })
        val key = ShareCase.decodeKey(share.url.substringAfter('#'))!!
        val c0 = upload.chunks.getValue(0)
        val swapped = ShareCase.aad(share.caseId.toBytes(), 1, 3, upload.request.bundleHash.toByteArray())
        assertNull(g.crypto.open(key, c0.copyOf(12), swapped, c0.copyOfRange(12, c0.size)))
    }

    @Test
    fun freeTierRejectsOversizedCases() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 2, bytesEach = 9_000_000)
        val tooLarge = assertIs<ShareOutcome.TooLarge>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY))
        assertEquals(Entitlement.FREE_CASE_BYTES, tooLarge.limit)
        assertTrue(g.media.uploads.isEmpty())
        g.entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, null)
        assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.NEVER))
        assertNull(g.media.uploads.values.single().request.expiresAtMs)
    }

    @Test
    fun plusCasesNeverExceedWhatTheGatewayCanUpload() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 2, bytesEach = 9_000_000)
        g.entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, null)
        g.media.maxCaseBytes = Entitlement.FREE_CASE_BYTES
        val tooLarge = assertIs<ShareOutcome.TooLarge>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY))
        assertEquals(Entitlement.FREE_CASE_BYTES, tooLarge.limit)
        assertTrue(g.media.uploads.isEmpty())
        g.media.maxCaseBytes = Entitlement.PLUS_CASE_BYTES
        assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY))
    }

    @Test
    fun onlyVerifiedMembersWithImagesAreShared() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 2)
        val other = g.manageCases.create("Other")
        val stranger = g.sealOne().also { g.captures.putImage(it.recordId, ByteString.of(byteArrayOf(1))) }
        val noImage = g.sealOne()
        assertIs<ShareOutcome.NothingToShare>(g.share(caseId, listOf(stranger.recordId, noImage.recordId), ShareExpiry.DAY))
        val shared = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId } + items[0].recordId + stranger.recordId, ShareExpiry.DAY))
        assertEquals(2, shared.share.itemCount)
        assertEquals(other.id, g.captures.get(stranger.recordId)!!.caseId)
    }

    @Test
    fun summaryCanBeExcluded() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 1)
        g.manageCases.updateSummary(caseId, "Two cars at Oak St")
        g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY, includeSummary = false)
        g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY)
        val uploads = g.media.uploads.values.toList()
        val key0 = ShareCase.decodeKey(g.cases.get(caseId)!!.shares[0].url.substringAfter('#'))!!
        val c = uploads[0].chunks.getValue(0)
        val plain = g.crypto.open(key0, c.copyOf(12), ShareCase.aad(uploads[0].request.caseId.toBytes(), 0, 1, uploads[0].request.bundleHash.toByteArray()), c.copyOfRange(12, c.size))!!
        assertNull(g.codec.decode(plain).summary)
        assertEquals(2, g.cases.get(caseId)!!.shares.size)
    }

    @Test
    fun caseIdUsesAMediaShardFromTheRegistry() = runTest {
        val g = TestGraph()
        g.registry.shardMap = ShardMap(listOf(ShardInfo(0, 0)), listOf(ShardInfo(4, 0), ShardInfo(7, 0)))
        val (caseId, items) = caseWith(g, 1)
        val shared = assertIs<ShareOutcome.Shared>(g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY))
        assertTrue(shared.share.caseId.shard in setOf(4, 7))
    }

    @Test
    fun failuresPropagateWithoutRecordingShare() = runTest {
        val g = TestGraph()
        val (caseId, items) = caseWith(g, 1)
        g.media.failPutAt = 0
        assertFailsWith<OfflineException> { g.share(caseId, items.map { it.recordId }, ShareExpiry.DAY) }
        assertTrue(g.cases.get(caseId)!!.shares.isEmpty())
        assertFailsWith<CaseException> { g.share(LocalCaseId(77), emptyList(), ShareExpiry.DAY) }
    }
}
