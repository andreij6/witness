package app.witness.core.model

enum class LocationPrecision(val geohashChars: Int?, val revealsPrecise: Boolean) {
    HIDDEN(null, false),
    AREA(5, false),
    NEIGHBORHOOD(6, true),
    EXACT(9, true),
}

data class BlurProposal(val rect: Rect, val blurClass: BlurClass, val enabled: Boolean = true, val confidence: Float = 1f)

data class PostDraft(
    val parent: RecordId,
    val dims: Dims,
    val blurs: List<BlurProposal>,
    val precision: LocationPrecision,
    val coarse: String?,
    val altText: String,
    val caption: String,
    val adjustments: List<Edit> = emptyList(),
    val aiGenerated: Boolean = false,
    val locationWarning: Boolean = false,
)

data class Post(
    val recordId: RecordId,
    val parent: RecordId,
    val edits: List<Edit>,
    val precision: LocationPrecision,
    val coarse: String?,
    val altText: String,
    val caption: String,
    val proofUrl: String,
)
