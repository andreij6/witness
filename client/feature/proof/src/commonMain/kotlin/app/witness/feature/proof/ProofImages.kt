package app.witness.feature.proof

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.ByteString
import app.witness.core.ui.rememberBitmap

@Composable
fun BytesImage(bytes: ByteString?, description: String?, modifier: Modifier = Modifier, contentScale: ContentScale = ContentScale.Crop) {
    val bitmap = rememberBitmap(bytes)
    Box(modifier.background(Witness.colors.surface2), contentAlignment = Alignment.Center) {
        if (bitmap != null) Image(bitmap, description, Modifier.fillMaxSize(), contentScale = contentScale)
        else WIcon(WitnessIcon.Image, Witness.colors.muted, size = 28.dp)
    }
}

@Composable
internal inline fun <reified VM : ViewModel> rememberViewModel(key: String, noinline create: () -> VM): VM {
    val owner = LocalViewModelStoreOwner.current ?: rememberLocalOwner()
    return viewModel(owner, key) { create() }
}

@Composable
@PublishedApi
internal fun rememberLocalOwner(): ViewModelStoreOwner {
    val owner = remember { object : ViewModelStoreOwner { override val viewModelStore = ViewModelStore() } }
    DisposableEffect(owner) { onDispose { owner.viewModelStore.clear() } }
    return owner
}
