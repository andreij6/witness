package app.witness.system

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.FileProvider
import app.witness.core.model.RecordId
import java.io.File

class SystemActions(private val context: Context) {
    fun shareText(text: String, title: String? = null) =
        start(Intent.createChooser(Intent(Intent.ACTION_SEND).setType(TEXT).putExtra(Intent.EXTRA_TEXT, text), title))

    fun openUrl(url: String) = start(Intent(Intent.ACTION_VIEW, Uri.parse(url)))

    fun copy(label: String, text: String) {
        context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText(label, text))
    }

    fun shareFile(name: String, mime: String, bytes: ByteArray, text: String? = null) {
        val dir = File(context.cacheDir, SHARED_DIR).apply { mkdirs() }
        val file = File(dir, safeName(name)).apply { writeBytes(bytes) }
        val uri = FileProvider.getUriForFile(context, context.packageName + AUTHORITY_SUFFIX, file)
        start(Intent.createChooser(sendStream(uri, mime, text), name))
    }

    private fun start(intent: Intent) {
        try {
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (_: ActivityNotFoundException) {
        }
    }

    companion object {
        const val TEXT: String = "text/plain"
        const val JPEG: String = "image/jpeg"

        fun sendStream(uri: Uri, mime: String, text: String?): Intent {
            val send = Intent(Intent.ACTION_SEND).setType(mime).putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (text != null) send.putExtra(Intent.EXTRA_TEXT, text)
            return send
        }

        fun postFileName(record: RecordId): String = "witness-${record.text}.jpg"
        const val SHARED_DIR: String = "shared"
        const val AUTHORITY_SUFFIX: String = ".files"

        fun safeName(name: String): String {
            val out = StringBuilder(minOf(name.length, MAX_NAME))
            for (c in name) {
                if (out.length == MAX_NAME) break
                out.append(if (c.isLetterOrDigit() || c == '.' || c == '-' || c == '_') c else '_')
            }
            return out.toString().trimStart('.').ifEmpty { "file" }
        }

        private const val MAX_NAME = 96
    }
}
