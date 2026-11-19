package app.witness.core.platform.share

import android.content.Intent
import android.net.Uri
import android.os.Build

object ShareIntents {
    fun parse(intent: Intent?): SharedItem? {
        if (intent == null) return null
        val streams = when (intent.action) {
            Intent.ACTION_SEND -> listOfNotNull(intent.stream())
            Intent.ACTION_SEND_MULTIPLE -> intent.streams()
            Intent.ACTION_VIEW -> return intent.dataString?.let { ShareParser.parse(null, it, emptyList()) }
            else -> return null
        }
        return ShareParser.parse(intent.type, intent.getStringExtra(Intent.EXTRA_TEXT), streams.map(Uri::toString))
    }

    fun deliver(intent: Intent?, inbox: ShareInbox): Boolean {
        val item = parse(intent) ?: return false
        inbox.offer(item)
        return true
    }

    @Suppress("DEPRECATION")
    private fun Intent.stream(): Uri? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else getParcelableExtra(Intent.EXTRA_STREAM)

    @Suppress("DEPRECATION")
    private fun Intent.streams(): List<Uri> =
        (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else getParcelableArrayListExtra(Intent.EXTRA_STREAM)).orEmpty()
}
