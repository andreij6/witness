package app.witness.web

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull

data class ShellSummary(val recordId: String?, val mediaShard: Int?) {
    companion object {
        const val ELEMENT_ID: String = "witness-shell"
        const val MAX_BYTES: Int = 16 * 1024

        fun parse(text: String?): ShellSummary? {
            if (text.isNullOrBlank() || text.length > MAX_BYTES) return null
            val obj = try {
                Json.parseToJsonElement(text) as? JsonObject
            } catch (_: IllegalArgumentException) {
                null
            } ?: return null
            val id = (obj["record_id"] as? JsonPrimitive)?.takeIf { it.isString }?.contentOrNull
                ?.takeIf { WebRoute.parse(WebRoute.PROOF + it) is WebRoute.Proof }
            val shard = (obj["media_shard"] as? JsonPrimitive)?.takeUnless { it.isString }?.intOrNull?.takeIf { it >= 0 }
            return ShellSummary(id, shard)
        }
    }
}
