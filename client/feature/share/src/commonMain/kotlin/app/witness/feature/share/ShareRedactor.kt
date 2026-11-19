package app.witness.feature.share

import app.witness.core.model.BlurClass
import app.witness.core.model.RecordId

data class Redaction(val blur: Set<BlurClass>, val hideLocation: Boolean) {
    val isEmpty: Boolean get() = blur.isEmpty() && !hideLocation
}

fun interface ShareRedactor {
    suspend fun redact(recordId: RecordId, redaction: Redaction): RecordId
}

enum class PrivacyOption(val label: String, val blurClass: BlurClass?) {
    BLUR_FACES("Blur faces", BlurClass.FACE),
    BLUR_PLATES("Blur plates", BlurClass.PLATE),
    HIDE_LOCATION("Hide location", null),
}

object RedactionPlan {
    fun forItem(detected: Set<BlurClass>, on: Set<PrivacyOption>): Redaction {
        var blur: MutableSet<BlurClass>? = null
        for (option in on) {
            val cls = option.blurClass ?: continue
            if (cls in detected) (blur ?: HashSet<BlurClass>(2).also { blur = it }).add(cls)
        }
        return Redaction(blur ?: emptySet(), PrivacyOption.HIDE_LOCATION in on)
    }

    fun counts(detections: Map<RecordId, Set<BlurClass>>, items: Collection<RecordId>): Map<BlurClass, Int> {
        val out = HashMap<BlurClass, Int>()
        for (id in items) for (cls in detections[id].orEmpty()) out[cls] = (out[cls] ?: 0) + 1
        return out
    }
}
