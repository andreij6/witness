package app.witness.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import app.witness.core.model.RecordId
import app.witness.feature.home.ThumbnailSource

@Composable
fun BoxScope.RecordThumbnail(source: ThumbnailSource, recordId: RecordId) {
    var sizePx by remember { mutableIntStateOf(0) }
    val bitmap by produceState<ImageBitmap?>(null, recordId, sizePx) {
        if (sizePx > 0) value = source.load(recordId, sizePx)
    }
    val image = bitmap
    val fill = Modifier.matchParentSize().onSizeChanged { sizePx = maxOf(it.width, it.height) }
    if (image == null) Spacer(fill)
    else Image(image, contentDescription = null, modifier = fill, contentScale = ContentScale.Crop)
}
