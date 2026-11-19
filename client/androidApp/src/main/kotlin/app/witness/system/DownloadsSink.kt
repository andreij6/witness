package app.witness.system

import android.content.ContentResolver
import android.content.ContentValues
import android.content.Context
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.feature.settings.DataExporter
import app.witness.feature.settings.ExportOutcome
import app.witness.feature.share.ExportOutput
import app.witness.feature.share.ExportSink
import app.witness.feature.share.ZipWriter
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withContext
import java.io.File
import java.io.OutputStream

class DownloadsSink(private val context: Context) : ExportSink {
    override suspend fun open(name: String, mimeType: String): ExportOutput = withContext(Dispatchers.IO) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) mediaStore(context.contentResolver, name, mimeType) else appFile(name)
    }

    private fun mediaStore(resolver: ContentResolver, name: String, mimeType: String): ExportOutput {
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, name)
            put(MediaStore.Downloads.MIME_TYPE, mimeType)
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        val uri = checkNotNull(resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)) { "could not create $name" }
        val out = checkNotNull(resolver.openOutputStream(uri)) { "could not open $name" }
        return StreamOutput(out, onCommit = {
            resolver.update(uri, ContentValues().apply { put(MediaStore.Downloads.IS_PENDING, 0) }, null, null)
            "Downloads/$name"
        }, onAbort = { resolver.delete(uri, null, null) })
    }

    private fun appFile(name: String): ExportOutput {
        val dir = context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS) ?: File(context.filesDir, Environment.DIRECTORY_DOWNLOADS)
        dir.mkdirs()
        val file = File(dir, name)
        return StreamOutput(file.outputStream(), onCommit = { file.absolutePath }, onAbort = { file.delete() })
    }
}

class StreamOutput(
    private val out: OutputStream,
    private val onCommit: () -> String,
    private val onAbort: () -> Unit,
) : ExportOutput {
    override suspend fun write(bytes: ByteArray, offset: Int, length: Int) = withContext(Dispatchers.IO) { out.write(bytes, offset, length) }

    override suspend fun commit(): String = withContext(Dispatchers.IO) {
        out.close()
        onCommit()
    }

    override suspend fun abort() = withContext(Dispatchers.IO) {
        runCatching { out.close() }
        onAbort()
    }
}

class CapturesExporter(private val captures: CaptureStore, private val sink: ExportSink, private val clock: Clock) : DataExporter {
    override suspend fun export(): ExportOutcome {
        val all = captures.observeAll().first()
        if (all.isEmpty()) return ExportOutcome.Ready(0)
        val out = sink.open(fileName(clock.nowEpochMillis()), MIME)
        return try {
            val zip = ZipWriter(out)
            var written = 0
            for (c in all) {
                val jpeg = captures.readImage(c.recordId) ?: continue
                zip.add(c.recordId.text + ".jpg", jpeg.toByteArray())
                written++
            }
            zip.finish()
            out.commit()
            ExportOutcome.Ready(written)
        } catch (e: Exception) {
            out.abort()
            if (e is CancellationException) throw e
            ExportOutcome.Failed(e.message ?: e::class.simpleName.orEmpty())
        }
    }

    companion object {
        const val MIME: String = "application/zip"

        fun fileName(nowMs: Long): String = "witness-export-$nowMs.zip"
    }
}
