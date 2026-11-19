package app.witness

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.lifecycleScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.model.CaptureMode
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.platform.share.ShareIntents
import app.witness.di.AppForeground
import app.witness.feature.check.readImage
import app.witness.nav.DeepLinks
import app.witness.nav.Outside
import app.witness.system.SystemActions
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity(), Outside {
    private val links = Channel<NavKey>(Channel.CONFLATED)
    private val app: WitnessApplication get() = application as WitnessApplication
    private val system by lazy { SystemActions(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val graph = app.graph
        setContent { WitnessApp(graph.features, this, links.receiveAsFlow()) }
        if (savedInstanceState == null) handle(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handle(intent)
    }

    override fun onStart() {
        super.onStart()
        AppForeground.onStart()
    }

    override fun onStop() {
        AppForeground.onStop()
        super.onStop()
    }

    private fun handle(intent: Intent?) {
        val item = ShareIntents.parse(intent) ?: return
        val shared = app.graph.features.sharedImages
        lifecycleScope.launch(Dispatchers.IO) {
            val key = DeepLinks.keyFor(item, { uri -> shared.stash { readImage(contentResolver, Uri.parse(uri)) } }, app.graph.features.links)
            if (key != null) links.send(key)
        }
    }

    override fun shareText(text: String) = system.shareText(text)

    override fun openUrl(url: String) = system.openUrl(url)

    override fun copy(text: String) = system.copy(LABEL, text)

    override fun shareFile(name: String, mime: String, bytes: ByteArray) = system.shareFile(name, mime, bytes)

    override fun sharePost(text: String, record: RecordId) {
        val features = app.graph.features
        lifecycleScope.launch {
            val jpeg = features.postImage(record)
            if (jpeg == null) system.shareText(text) else system.shareFile(SystemActions.postFileName(record), SystemActions.JPEG, jpeg, text)
        }
    }

    override fun captureInto(case: LocalCaseId) {
        val features = app.graph.features
        lifecycleScope.launch {
            features.manage.setActive(case)
            features.settingsStore.update { it.copy(captureMode = CaptureMode.CASE) }
        }
    }

    override fun finish() = super.finish()

    private companion object {
        const val LABEL = "Witness link"
    }
}
