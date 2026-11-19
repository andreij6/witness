package app.witness.core.platform.share

import app.witness.core.model.RecordId
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

sealed interface SharedItem {
    data class Images(val uris: List<String>) : SharedItem
    data class ProofLink(val url: String, val recordId: RecordId) : SharedItem
    data class Link(val url: String) : SharedItem
}

object ShareParser {
    const val MAX_IMAGES = 20
    private const val PROOF_SEGMENT = "/p/"

    fun parse(mimeType: String?, text: String?, streams: List<String>): SharedItem? {
        val images = streams.asSequence().filter(::isContentUri).distinct().take(MAX_IMAGES).toList()
        if (mimeType != null && mimeType.startsWith("image/") && images.isNotEmpty()) {
            return SharedItem.Images(images)
        }
        val url = text?.let(::firstUrl) ?: return null
        return proofRecord(url)?.let { SharedItem.ProofLink(url, it) } ?: SharedItem.Link(url)
    }

    fun isContentUri(uri: String): Boolean = uri.startsWith(CONTENT_SCHEME, ignoreCase = true) && uri.length > CONTENT_SCHEME.length

    fun firstUrl(text: String): String? {
        val start = indexOfScheme(text)
        if (start < 0) return null
        var end = start
        while (end < text.length && !text[end].isWhitespace() && text[end] != '"' && text[end] != '<' && text[end] != '>') end++
        while (end > start && text[end - 1] in TRAILING) end--
        val url = text.substring(start, end)
        return url.takeIf { it.length > url.indexOf("://") + 3 }
    }

    fun proofRecord(url: String): RecordId? {
        val at = url.indexOf(PROOF_SEGMENT, startIndex = url.indexOf("://").coerceAtLeast(0) + 3)
        if (at < 0) return null
        val from = at + PROOF_SEGMENT.length
        var to = from
        while (to < url.length && url[to] != '/' && url[to] != '?' && url[to] != '#') to++
        return RecordId.parseOrNull(url.substring(from, to))
    }

    private fun indexOfScheme(text: String): Int {
        val https = text.indexOf("https://", ignoreCase = true)
        val http = text.indexOf("http://", ignoreCase = true)
        return when {
            https < 0 -> http
            http < 0 -> https
            else -> minOf(https, http)
        }
    }

    private const val TRAILING = ".,;:!?)]}'"
    private const val CONTENT_SCHEME = "content://"
}

class ShareInbox {
    private val pending = MutableStateFlow<SharedItem?>(null)
    val incoming: StateFlow<SharedItem?> get() = pending

    fun offer(item: SharedItem) {
        pending.value = item
    }

    fun consume(): SharedItem? = pending.getAndSet(null)

    private fun <T> MutableStateFlow<T>.getAndSet(value: T): T {
        while (true) {
            val current = this.value
            if (compareAndSet(current, value)) return current
        }
    }
}
