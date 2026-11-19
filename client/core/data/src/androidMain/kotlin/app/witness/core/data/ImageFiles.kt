package app.witness.core.data

import app.witness.core.model.ByteString
import app.witness.core.model.RecordId
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

class ImageFiles(private val dir: File, private val io: CoroutineDispatcher = Dispatchers.IO) {
    fun file(recordId: RecordId): File = File(dir, recordId.text + EXT)

    suspend fun read(recordId: RecordId): ByteString? = withContext(io) {
        file(recordId).takeIf(File::isFile)?.readBytes()?.let(ByteString::wrap)
    }

    suspend fun write(recordId: RecordId, jpeg: ByteString) = withContext(io) {
        dir.mkdirs()
        val target = file(recordId)
        val tmp = File(dir, recordId.text + TMP)
        tmp.outputStream().use { out -> out.write(jpeg.toByteArray()) }
        check(tmp.renameTo(target)) { "could not persist image" }
    }

    suspend fun rename(from: RecordId, to: RecordId) = withContext(io) {
        val source = file(from)
        if (source.isFile) check(source.renameTo(file(to))) { "could not rename image" }
    }

    suspend fun delete(recordId: RecordId) = withContext(io) { file(recordId).delete() }

    companion object {
        const val DIR = "captures"
        private const val EXT = ".jpg"
        private const val TMP = ".tmp"
    }
}
