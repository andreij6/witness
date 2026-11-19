package app.witness.feature.post

import android.content.ClipData
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.Clipboard
import androidx.compose.ui.platform.LocalClipboard
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Post
import app.witness.core.model.RecordId
import kotlinx.coroutines.launch

fun EntryProviderScope<NavKey>.postEntries(
    newComposerViewModel: (RecordId) -> ComposerViewModel,
    newSharedViewModel: (RecordId, LocationPrecision) -> PostSharedViewModel,
    onClose: () -> Unit,
    onShare: (text: String, post: Post) -> Unit,
    onShared: (PostSharedKey) -> Unit,
    onDone: () -> Unit,
) {
    entry<PostComposerKey> { key ->
        ComposerRoute(viewModel(key = key.parent) { newComposerViewModel(key.recordId) }, onClose, onShare, onShared)
    }
    entry<PostSharedKey> { key ->
        PostSharedRoute(viewModel(key = key.record) { newSharedViewModel(key.recordId, key.precision) }, onDone)
    }
}

suspend fun Clipboard.copyProofLink(url: String) = setClipEntry(ClipEntry(ClipData.newPlainText("Proof link", url)))

@Composable
fun ComposerRoute(vm: ComposerViewModel, onClose: () -> Unit, onShare: (String, Post) -> Unit, onShared: (PostSharedKey) -> Unit) {
    val state by vm.state.collectAsStateWithLifecycle()
    val clipboard = LocalClipboard.current
    LaunchedEffect(vm) {
        vm.effects.collect { effect ->
            when (effect) {
                is ComposerEffect.CopyLink -> clipboard.copyProofLink(effect.url)
                is ComposerEffect.Share -> {
                    onShare(effect.text, effect.post)
                    onShared(PostSharedKey.of(effect.post.recordId, effect.post.precision))
                }
            }
        }
    }
    ComposerScreen(state, vm, onClose)
}

@Composable
fun PostSharedRoute(vm: PostSharedViewModel, onDone: () -> Unit) {
    val state by vm.state.collectAsStateWithLifecycle()
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    PostSharedScreen(state, { url -> scope.launch { clipboard.copyProofLink(url) } }, onDone)
}
