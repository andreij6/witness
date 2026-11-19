package app.witness.nav

import androidx.navigation3.runtime.NavKey
import app.witness.core.domain.port.Links
import app.witness.core.platform.share.SharedItem
import app.witness.feature.check.CheckKey
import app.witness.feature.check.CheckSharedKey
import app.witness.feature.check.PastedLink
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ProofPageKey

object DeepLinks {
    const val HOST: String = "witness.app"

    fun keyFor(url: String, links: Links = Links()): NavKey? {
        if (isCheck(url)) return CheckKey
        return when (val link = PastedLink.parse(url, links)) {
            is PastedLink.Proof -> ProofPageKey(link.recordId.text)
            is PastedLink.Case -> CaseLinkKey(link.caseId.text, link.fragment)
            else -> null
        }
    }

    fun keyFor(item: SharedItem, stash: (String) -> String?, links: Links = Links()): NavKey? = when (item) {
        is SharedItem.ProofLink -> keyFor(item.url, links)
        is SharedItem.Link -> keyFor(item.url, links)
        is SharedItem.Images -> item.uris.firstOrNull()?.let(stash)?.let(::CheckSharedKey)
    }

    fun isCheck(url: String): Boolean = "://" in url &&
        url.substringAfter("://").removePrefix("www.").substringBefore('?').substringBefore('#').trimEnd('/').equals("$HOST/check", ignoreCase = true)
}
