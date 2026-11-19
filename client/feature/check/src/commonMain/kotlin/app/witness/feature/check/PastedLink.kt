package app.witness.feature.check

import app.witness.core.domain.port.Links
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.model.CaseId
import app.witness.core.model.RecordId

sealed interface PastedLink {
    data class Proof(val recordId: RecordId) : PastedLink

    class Case(val caseId: CaseId, val fragment: String) : PastedLink {
        override fun equals(other: Any?): Boolean = other is Case && caseId == other.caseId && fragment == other.fragment

        override fun hashCode(): Int = caseId.hashCode()

        override fun toString(): String = "Case($caseId, <key>)"
    }

    enum class Problem(val message: String) : PastedLink {
        Empty("Paste a Witness proof or case link"),
        Foreign("That isn't a Witness link. Save the image and choose it instead."),
        Malformed("This Witness link looks incomplete. Copy it again."),
        MissingKey("This case link has no key. Ask the sender for the full link."),
    }

    companion object {
        const val MAX_LENGTH: Int = 2048

        fun parse(text: String, links: Links = Links()): PastedLink {
            val raw = text.trim()
            if (raw.isEmpty()) return Problem.Empty
            if (raw.length > MAX_LENGTH) return Problem.Malformed
            val proofPrefix = hostPath(links.proofBase)
            val casePrefix = hostPath(links.caseBase)
            val bare = hostPath(raw)
            return when {
                bare.startsWith(proofPrefix, ignoreCase = true) -> proof(bare.substring(proofPrefix.length))
                bare.startsWith(casePrefix, ignoreCase = true) -> case(bare.substring(casePrefix.length))
                isUrl(raw) -> Problem.Foreign
                else -> RecordId.parseOrNull(raw.replace("-", ""))?.let(::Proof) ?: Problem.Foreign
            }
        }

        private fun proof(rest: String): PastedLink {
            val id = rest.substringBefore('#').substringBefore('?').trimEnd('/')
            return RecordId.parseOrNull(id.replace("-", ""))?.let(::Proof) ?: Problem.Malformed
        }

        private fun case(rest: String): PastedLink {
            val hash = rest.indexOf('#')
            val path = (if (hash < 0) rest else rest.substring(0, hash)).substringBefore('?').trimEnd('/')
            val caseId = CaseId.parseOrNull(path.replace("-", "")) ?: return Problem.Malformed
            if (hash < 0) return Problem.MissingKey
            val fragment = rest.substring(hash + 1)
            return if (ShareCase.decodeKey(fragment) != null) Case(caseId, fragment) else Problem.MissingKey
        }

        private fun hostPath(url: String): String = url.substringAfter("://").removePrefix("www.")

        private fun isUrl(text: String): Boolean = "://" in text || text.startsWith("www.") || (text.contains('.') && text.contains('/'))
    }
}
