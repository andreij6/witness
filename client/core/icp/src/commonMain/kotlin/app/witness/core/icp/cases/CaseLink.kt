package app.witness.core.icp.cases

import app.witness.core.icp.id.CaseId
import kotlin.io.encoding.Base64
import kotlin.io.encoding.ExperimentalEncodingApi

@OptIn(ExperimentalEncodingApi::class)
class CaseLink(val caseId: CaseId, key: ByteArray) {
    init {
        require(key.size == CaseChunks.KEY_BYTES) { "case key needs ${CaseChunks.KEY_BYTES} bytes" }
    }

    private val secret = key.copyOf()

    val key: ByteArray get() = secret.copyOf()

    fun url(origin: String = ORIGIN): String = "$origin$PATH${caseId.text}#$KEY_PARAM${base64.encode(secret)}"

    override fun equals(other: Any?): Boolean =
        other is CaseLink && caseId == other.caseId && secret.contentEquals(other.secret)

    override fun hashCode(): Int = caseId.hashCode()

    override fun toString(): String = "CaseLink(caseId=$caseId, key=<redacted>)"

    companion object {
        const val ORIGIN: String = "https://case.witness.app"
        const val PATH: String = "/v/"
        const val KEY_PARAM: String = "k="

        private val base64 = Base64.UrlSafe.withPadding(Base64.PaddingOption.ABSENT_OPTIONAL)

        fun parse(url: String): CaseLink? {
            val hash = url.indexOf('#')
            if (hash < 0) return null
            val beforeFragment = url.substring(0, hash).substringBefore('?')
            return parse(beforeFragment, url.substring(hash + 1))
        }

        fun parse(path: String, fragment: String): CaseLink? {
            val at = path.lastIndexOf(PATH)
            if (at < 0) return null
            val caseId = CaseId.parse(path.substring(at + PATH.length).trimEnd('/')) ?: return null
            val key = keyFrom(fragment.removePrefix("#")) ?: return null
            return CaseLink(caseId, key)
        }

        fun keyFrom(fragment: String): ByteArray? {
            val param = fragment.split('&').firstOrNull { it.startsWith(KEY_PARAM) } ?: return null
            val key = try {
                base64.decode(param.substring(KEY_PARAM.length))
            } catch (_: IllegalArgumentException) {
                return null
            }
            return key.takeIf { it.size == CaseChunks.KEY_BYTES }
        }
    }
}
