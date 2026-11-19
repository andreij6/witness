package app.witness.core.icp.witness

import app.witness.core.icp.IcpException
import app.witness.core.icp.web.toByteArray
import app.witness.core.icp.web.toUint8Array
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.await
import org.khronos.webgl.Uint8Array
import kotlin.js.JsString
import app.witness.core.icp.witness.check as jsCheck
import app.witness.core.icp.witness.key as jsKey
import app.witness.core.icp.witness.openCase as jsOpenCase
import app.witness.core.icp.witness.preview as jsPreview
import app.witness.core.icp.witness.proof as jsProof

object JsWitnessBridge : WitnessBridge {
    override suspend fun check(input: ByteArray, listener: StageListener): String = call {
        jsCheck(input.toUint8Array()) { s, st, done, total -> listener.onStage(s, st, done, total) }.await<JsString>().toString()
    }

    override suspend fun proof(recordId: String): String? = call { jsProof(recordId).await<JsString?>()?.toString() }

    override suspend fun key(keyIdHex: String): String? = call { jsKey(keyIdHex).await<JsString?>()?.toString() }

    override suspend fun preview(recordId: String): ByteArray? = call { jsPreview(recordId).await<Uint8Array?>()?.toByteArray() }

    override suspend fun openCase(caseId: String, fragment: String): OpenedBundle = call {
        val js = jsOpenCase(caseId, fragment).await<CaseJs>()
        OpenedBundle(js.status, js.json, js.bundle?.toByteArray())
    }

    private suspend inline fun <T> call(block: () -> T): T = try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        throw IcpException("witness bridge: ${e.message}", e)
    }
}
