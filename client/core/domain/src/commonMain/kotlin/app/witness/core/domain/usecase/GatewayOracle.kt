package app.witness.core.domain.usecase

import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.PdqHit
import app.witness.core.domain.port.PdqProbe
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ProbeResult
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CheckStage
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.RecordId
import app.witness.core.model.StageState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class GatewayOracle(
    private val registry: RegistryGateway,
    private val index: IndexGateway,
    private val media: MediaGateway,
    private val options: JudgeOptions,
    private val stages: StageSink = StageSink { _, _, _, _, _ -> },
) : VerifyOracle {
    private val lock = Mutex()
    private val keys = HashMap<KeyId, KeyInfo?>()
    private val records = HashMap<RecordId, AnchoredRecord?>()
    private val byIx = HashMap<Long, AnchoredRecord?>()
    private var shardList: List<ShardInfo>? = null

    override suspend fun key(keyId: KeyId): KeyInfo? = memo(keys, keyId) { registry.getKey(it) }

    override suspend fun record(recordId: RecordId): AnchoredRecord? = memo(records, recordId) { registry.get(it) }

    override suspend fun exact(assetHash: ByteString, pixelHash: ByteString?): List<AnchoredRecord> {
        val (results, _) = fanOut(ProbeQuery(exact = listOfNotNull(assetHash, pixelHash)), report = false)
        val ixs = LinkedHashSet<Long>()
        for (r in results) ixs.addAll(r.exact)
        return resolve(ixs.toList()).sortedWith(EARLIEST).take(options.maxExact)
    }

    override suspend fun byWatermark(captureId: CaptureId): List<AnchoredRecord> = coroutineScope {
        registry.byCapture(captureId).distinct().take(options.maxWatermark)
            .map { async { record(it) } }.awaitAll().filterNotNull().sortedWith(EARLIEST)
    }

    override suspend fun pdq(hashes: List<ByteString>): PdqProbe {
        val (results, asked) = fanOut(ProbeQuery(pdq = hashes.take(options.maxQueryHashes)), report = true)
        val best = HashMap<Long, Int>()
        var truncated = false
        for (r in results) {
            truncated = truncated || r.truncated
            for (p in r.pdq) {
                val prev = best[p.recordIx]
                if (prev == null || p.distance < prev) best[p.recordIx] = p.distance
            }
        }
        val ranked = rankByDistance(best, options.maxPdqCandidates)
        val resolved = resolve(ranked.map { it.first })
        val byIxResolved = resolved.associateBy { it.recordIx }
        val hits = ranked.mapNotNull { (ix, d) -> byIxResolved[ix]?.let { PdqHit(it, d) } }
        return PdqProbe(hits, truncated || results.size < asked, asked, results.size)
    }

    override suspend fun derivatives(root: AnchoredRecord): List<AnchoredRecord> {
        val shard = shardFor(shards(), root.recordIx) ?: return emptyList()
        val ixs = guard { index.derivatives(shard, root.recordIx) } ?: return emptyList()
        return resolve(ixs.distinct().take(options.maxDerivatives)).sortedWith(EARLIEST)
    }

    override suspend fun reference(record: AnchoredRecord): ByteString? = guard { media.reference(record.recordId) }

    private suspend fun resolve(ixs: List<Long>): List<AnchoredRecord> = coroutineScope {
        ixs.map { ix -> async { memo(byIx, ix) { registry.getByIndex(it) } } }.awaitAll().filterNotNull()
    }

    private suspend fun shards(): List<ShardInfo> =
        shardList ?: registry.shards().index.sortedBy { it.baseIx }.also { shardList = it }

    private suspend fun fanOut(query: ProbeQuery, report: Boolean): Pair<List<ProbeResult>, Int> = coroutineScope {
        val all = shards()
        var answered = 0
        val sinkLock = Mutex()
        if (report) stages.update(CheckStage.FINGERPRINTS, StageState.RUNNING, 0, all.size, null)
        val results = all.map { shard ->
            async {
                guard { index.probe(shard, query) }.also { r ->
                    if (report && r != null) sinkLock.withLock { stages.update(CheckStage.FINGERPRINTS, StageState.RUNNING, ++answered, all.size, null) }
                }
            }
        }.awaitAll().filterNotNull()
        results to all.size
    }

    private suspend fun <K, V> memo(map: HashMap<K, V?>, key: K, load: suspend (K) -> V?): V? {
        lock.withLock { if (map.containsKey(key)) return map[key] }
        val v = load(key)
        lock.withLock { map[key] = v }
        return v
    }

    private suspend fun <T> guard(block: suspend () -> T): T? =
        try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }

    companion object {
        val EARLIEST: Comparator<AnchoredRecord> = compareBy<AnchoredRecord>({ it.anchoredAtMs }, { it.recordIx })

        fun rankByDistance(best: Map<Long, Int>, limit: Int): List<Pair<Long, Int>> =
            best.entries.map { it.key to it.value }
                .sortedWith(compareBy<Pair<Long, Int>>({ it.second }, { it.first }))
                .take(limit)

        fun shardFor(sortedShards: List<ShardInfo>, recordIx: Long): ShardInfo? {
            var lo = 0
            var hi = sortedShards.size
            while (lo < hi) {
                val mid = (lo + hi) ushr 1
                if (sortedShards[mid].baseIx <= recordIx) lo = mid + 1 else hi = mid
            }
            return sortedShards.getOrNull(lo - 1)
        }
    }
}
