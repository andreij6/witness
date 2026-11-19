package app.witness.core.verify

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.first

fun interface VerifyEngine {
    fun check(input: ByteArray): Flow<CheckUpdate>
}

suspend fun VerifyEngine.judge(input: ByteArray): Judgement =
    check(input).filterIsInstance<CheckUpdate.Done>().first().judgement

expect fun createVerifyEngine(config: VerifyConfig = VerifyConfig()): VerifyEngine

data class VerifyConfig(val engineUrl: String? = null)
