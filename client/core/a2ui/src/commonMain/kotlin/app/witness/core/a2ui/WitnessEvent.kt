package app.witness.core.a2ui

sealed interface WitnessEvent {
    val surfaceId: String
    val target: String

    data class CaptureFor(override val surfaceId: String, override val target: String) : WitnessEvent
    data class Edit(override val surfaceId: String, override val target: String) : WitnessEvent
    data class Remove(override val surfaceId: String, override val target: String) : WitnessEvent
    data class Flag(override val surfaceId: String, override val target: String) : WitnessEvent
}

class WitnessEventSink(private val onEvent: (WitnessEvent) -> Unit) {
    fun dispatch(surfaceId: String, name: String, context: Map<String, Any?>): Boolean {
        val event = parse(surfaceId, name, context) ?: return false
        onEvent(event)
        return true
    }

    companion object {
        private const val MAX_TARGET_LENGTH = 64

        fun parse(surfaceId: String, name: String, context: Map<String, Any?>): WitnessEvent? {
            val target = (context["target"] as? String)?.takeIf(::isTarget) ?: return null
            return when (name) {
                WitnessCatalog.CAPTURE_FOR -> WitnessEvent.CaptureFor(surfaceId, target)
                WitnessCatalog.EDIT -> WitnessEvent.Edit(surfaceId, target)
                WitnessCatalog.REMOVE -> WitnessEvent.Remove(surfaceId, target)
                WitnessCatalog.FLAG -> WitnessEvent.Flag(surfaceId, target)
                else -> null
            }
        }

        fun targetOf(action: Map<String, Any?>?): String? =
            ((action?.get("event") as? Map<*, *>)?.get("context") as? Map<*, *>)?.get("target") as? String

        private fun isTarget(value: String): Boolean = value.length <= MAX_TARGET_LENGTH && TemplateCodec.isIdentifier(value)
    }
}
