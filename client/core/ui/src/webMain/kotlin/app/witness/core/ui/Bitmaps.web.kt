package app.witness.core.ui

import androidx.compose.ui.graphics.ImageBitmap
import org.jetbrains.compose.resources.decodeToImageBitmap

actual fun decodeBitmap(bytes: ByteArray, maxDimension: Int): ImageBitmap? = bytes.decodeToImageBitmap()
