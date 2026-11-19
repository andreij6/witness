package app.witness.core.a2ui

import java.io.File

internal object BundledTemplates {
    private val root = File("src/commonMain/composeResources")

    fun read(path: String): ByteArray = File(root, path).readBytes()

    suspend fun store(): TemplateStore = TemplateStore.bundled { read(it) }
}
