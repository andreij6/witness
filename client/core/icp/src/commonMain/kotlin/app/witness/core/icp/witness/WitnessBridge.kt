package app.witness.core.icp.witness

class OpenedBundle(val status: String, val json: String?, val bundle: ByteArray?) {
    companion object {
        const val OPENED: String = "opened"
        const val NOT_FOUND: String = "not_found"
        const val TAMPERED: String = "tampered"
    }
}

fun interface StageListener {
    fun onStage(stage: Int, state: Int, completed: Int, total: Int)
}

interface WitnessBridge {
    suspend fun check(input: ByteArray, listener: StageListener): String

    suspend fun proof(recordId: String): String?

    suspend fun key(keyIdHex: String): String?

    suspend fun preview(recordId: String): ByteArray?

    suspend fun openCase(caseId: String, fragment: String): OpenedBundle
}
