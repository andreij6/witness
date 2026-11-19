@file:JsModule("witness-bridges/icp.mjs")

package app.witness.core.icp.web

import org.khronos.webgl.Int32Array
import org.khronos.webgl.Uint8Array
import kotlin.js.JsAny
import kotlin.js.JsArray
import kotlin.js.JsString
import kotlin.js.Promise

external interface IcpClientJs : JsAny

external interface RecordJs : JsAny {
    val recordIx: String
    val recordId: String
    val claim: Uint8Array
    val c2paClaim: Uint8Array
    val cose: Uint8Array
    val anchoredNs: String
    val mmrProof: JsArray<Uint8Array>
    val leafCount: String
    val certificate: Uint8Array
}

external interface ShardJs : JsAny {
    val canisterId: String
    val baseIx: String
    val indexedUpto: String
}

external interface ShardMapJs : JsAny {
    val index: JsArray<ShardJs>
    val media: JsArray<ShardJs>
}

external interface CaseMetaJs : JsAny {
    val count: Int
    val bytes: Double
    val bundleHash: Uint8Array
    val expiresMs: Double
}

external fun connect(host: String, fetchRootKey: Boolean, registryId: String): Promise<IcpClientJs>

external fun getRecord(client: IcpClientJs, recordId: String): Promise<RecordJs?>

external fun byCapture(client: IcpClientJs, captureHi: Int, captureLo: Int): Promise<JsString>

external fun shards(client: IcpClientJs): Promise<ShardMapJs>

external fun probeAll(
    client: IcpClientJs,
    canisterIds: String,
    pdq: Uint8Array,
    watermarks: Int32Array,
    exact: Uint8Array,
    exactStride: Int,
    onShard: (Int, Int) -> Unit,
): Promise<Int32Array>

external fun caseMeta(client: IcpClientJs, mediaId: String, caseId: Uint8Array): Promise<CaseMetaJs?>

external fun caseChunk(client: IcpClientJs, mediaId: String, caseId: Uint8Array, ix: Int): Promise<Uint8Array?>
