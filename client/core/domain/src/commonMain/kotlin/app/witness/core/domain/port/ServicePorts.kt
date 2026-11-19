package app.witness.core.domain.port

import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureMode
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateSchema

data class AssistantContext(
    val mode: CaptureMode,
    val title: String? = null,
    val place: String? = null,
    val detected: List<BlurClass> = emptyList(),
    val records: List<RecordId> = emptyList(),
)

interface Assistant {
    suspend fun isAvailable(): Boolean

    suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion?
}

fun interface TemplateCatalog {
    suspend fun templates(): List<TemplateSchema>
}

interface Crypto {
    suspend fun sha256(data: ByteArray, offset: Int = 0, length: Int = data.size - offset): ByteArray

    suspend fun aesGcmSeal(key: ByteArray, nonce: ByteArray, aad: ByteArray, plaintext: ByteArray, offset: Int, length: Int): ByteArray

    companion object {
        const val KEY_SIZE: Int = 32
        const val NONCE_SIZE: Int = 12
        const val TAG_SIZE: Int = 16
    }
}

data class CaseBundleItem(val recordId: RecordId, val jpeg: ByteString, val submission: Submission?)

data class CaseBundle(val title: String, val summary: String?, val items: List<CaseBundleItem>)

fun interface CaseBundleCodec {
    fun encode(bundle: CaseBundle): ByteArray
}

enum class AnalyticsEvent {
    CAPTURE_SEALED,
    POST_PUBLISHED,
    CASE_CREATED,
    CASE_SHARED,
    CHECK_COMPLETED,
    ANCHOR_RUN,
    KEY_ENROLLED,
    KEY_ROTATED,
    ERASE_REQUESTED,
}

interface Telemetry {
    fun setCollectionEnabled(enabled: Boolean)

    fun event(event: AnalyticsEvent, metrics: Map<String, Long> = emptyMap())

    fun breadcrumb(message: String)

    fun nonFatal(error: Throwable)

    object None : Telemetry {
        override fun setCollectionEnabled(enabled: Boolean) = Unit

        override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) = Unit

        override fun breadcrumb(message: String) = Unit

        override fun nonFatal(error: Throwable) = Unit
    }
}

fun interface RandomSource {
    fun nextBytes(size: Int): ByteArray
}

data class Links(
    val proofBase: String = "https://witness.app/p/",
    val caseBase: String = "https://case.witness.app/v/",
) {
    fun proof(recordId: RecordId): String = proofBase + recordId.text

    companion object {
        fun at(origin: String): Links {
            val base = origin.trim().trimEnd('/')
            return if (base.isEmpty()) Links() else Links("$base/p/", "$base/v/")
        }
    }
}
