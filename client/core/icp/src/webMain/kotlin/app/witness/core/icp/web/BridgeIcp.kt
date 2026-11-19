package app.witness.core.icp.web

import app.witness.core.icp.IcpEndpoint
import app.witness.core.icp.IcpException
import app.witness.core.icp.IcpWire
import app.witness.core.icp.cases.CaseMeta
import app.witness.core.icp.cases.CaseStore
import app.witness.core.icp.id.CaptureId
import app.witness.core.icp.id.CaseId
import app.witness.core.icp.id.RecordId
import app.witness.core.icp.index.IndexSearch
import app.witness.core.icp.index.ProbeQuery
import app.witness.core.icp.index.ProbeWire
import app.witness.core.icp.index.ShardProbe
import app.witness.core.icp.index.ShardProgress
import app.witness.core.icp.registry.CertifiedRecord
import app.witness.core.icp.registry.RegistryGateway
import app.witness.core.icp.registry.ShardInfo
import app.witness.core.icp.registry.ShardMap
import app.witness.core.icp.web.byCapture as jsByCapture
import app.witness.core.icp.web.shards as jsShards
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.await
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.khronos.webgl.Int32Array
import org.khronos.webgl.Uint8Array
import kotlin.js.JsArray
import kotlin.js.JsString

class BridgeIcp(private val endpoint: IcpEndpoint) : RegistryGateway, IndexSearch, CaseStore {
    private val lock = Mutex()
    private var client: IcpClientJs? = null
    private var shardMap: ShardMap? = null

    private suspend fun client(): IcpClientJs = lock.withLock {
        client ?: call { connect(endpoint.host, endpoint.fetchRootKey, endpoint.registryId).await<IcpClientJs>() }.also { client = it }
    }

    override suspend fun get(recordId: RecordId): CertifiedRecord? {
        val c = client()
        return call { getRecord(c, recordId.text).await<RecordJs?>() }?.let(::record)
    }

    override suspend fun byCapture(captureId: CaptureId): List<RecordId> {
        val c = client()
        return IcpWire.recordIds(call { jsByCapture(c, captureId.high, captureId.low).await<JsString>() }.toString())
    }

    override suspend fun shards(): ShardMap {
        shardMap?.let { return it }
        val c = client()
        val js = call { jsShards(c).await<ShardMapJs>() }
        return ShardMap.of(js.index.toShards(), js.media.toShards()).also { shardMap = it }
    }

    override suspend fun probe(shards: List<ShardInfo>, query: ProbeQuery, progress: ShardProgress): List<ShardProbe> {
        val c = client()
        val wire = call {
            probeAll(
                c,
                IcpWire.joinIds(shards),
                query.pdq.toUint8Array(),
                query.watermarkPairs().toJsInt32Array(),
                query.exact.toUint8Array(),
                query.exactStride,
            ) { answered, total -> progress.onShard(answered, total) }.await<Int32Array>()
        }
        val probes = ProbeWire.decode(wire.toKotlinIntArray())
        if (probes.size != shards.size) throw IcpException("${probes.size} probe results for ${shards.size} shards")
        return probes
    }

    private suspend fun mediaId(caseId: CaseId): String =
        shards().mediaShard(caseId.shard)?.canisterId ?: throw IcpException("no media shard ${caseId.shard}")

    override suspend fun meta(caseId: CaseId): CaseMeta? {
        val c = client()
        val media = mediaId(caseId)
        val js = call { caseMeta(c, media, caseId.bytes.toUint8Array()).await<CaseMetaJs?>() } ?: return null
        return CaseMeta(js.count, js.bytes.toLong(), js.bundleHash.toByteArray(), js.expiresMs.toLong())
    }

    override suspend fun chunk(caseId: CaseId, ix: Int): ByteArray? {
        val c = client()
        val media = mediaId(caseId)
        return call { caseChunk(c, media, caseId.bytes.toUint8Array(), ix).await<Uint8Array?>() }?.toByteArray()
    }

    private fun record(js: RecordJs): CertifiedRecord = CertifiedRecord(
        recordIx = IcpWire.u64(js.recordIx),
        recordId = IcpWire.recordId(js.recordId),
        claim = js.claim.toByteArray(),
        c2paClaim = js.c2paClaim.toByteArray(),
        cose = js.cose.toByteArray(),
        anchoredNs = IcpWire.u64(js.anchoredNs),
        mmrProof = List(js.mmrProof.length) { js.mmrProof[it]!!.toByteArray() },
        leafCount = IcpWire.u64(js.leafCount),
        certificate = js.certificate.toByteArray(),
    )

    private fun JsArray<ShardJs>.toShards(): List<ShardInfo> =
        List(length) { i -> this[i]!!.let { IcpWire.shard(it.canisterId, it.baseIx, it.indexedUpto) } }

    private suspend inline fun <T> call(block: () -> T): T = try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: IcpException) {
        throw e
    } catch (e: Throwable) {
        throw IcpException("icp bridge call failed: ${e.message}", e)
    }
}
