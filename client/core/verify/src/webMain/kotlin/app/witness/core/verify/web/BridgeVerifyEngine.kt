package app.witness.core.verify.web

import app.witness.core.icp.web.toUint8Array
import app.witness.core.verify.CheckUpdate
import app.witness.core.verify.Judgement
import app.witness.core.verify.VerifyEngine
import app.witness.core.verify.VerifyWire
import kotlinx.coroutines.await
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow

class BridgeVerifyEngine(private val engineUrl: String?) : VerifyEngine {
    override fun check(input: ByteArray): Flow<CheckUpdate> = callbackFlow {
        val result = judge(engineUrl, input.toUint8Array()) { stage, state, answered, total ->
            VerifyWire.progress(stage, state, answered, total)?.let { trySend(it) }
        }.await<JudgementJs>()
        send(CheckUpdate.Done(result.toJudgement()))
        close()
        awaitClose()
    }

    suspend fun isFallback(): Boolean = isFallback(engineUrl).await<kotlin.js.JsBoolean>().toBoolean()

    private fun JudgementJs.toJudgement(): Judgement = VerifyWire.judgement(verdict, recordId, flags, fallback)
}
