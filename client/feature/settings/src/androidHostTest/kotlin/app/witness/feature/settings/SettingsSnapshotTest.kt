package app.witness.feature.settings

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.WitnessTheme
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import org.robolectric.shadows.ShadowLooper
import java.io.File
import kotlin.test.Test
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w412dp-h915dp-xxhdpi")
class SettingsSnapshotTest {
    private fun snap(name: String, content: @Composable () -> Unit) {
        val controller = Robolectric.buildActivity(ComponentActivity::class.java)
        controller.get().setTheme(android.R.style.Theme_Material_Light_NoActionBar)
        val activity = controller.setup().get()
        activity.setContent { WitnessTheme { content() } }
        repeat(5) { ShadowLooper.idleMainLooper() }
        val root: View = activity.window.decorView
        val bitmap = Bitmap.createBitmap(root.width, root.height, Bitmap.Config.ARGB_8888)
        root.draw(Canvas(bitmap))
        val dir = File(System.getProperty("snapshotDir") ?: "build/snapshots").apply { mkdirs() }
        File(dir, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        assertTrue(root.width > 0 && root.height > 0)
    }

    @Test
    @Config(qualifiers = "w412dp-h1250dp-xxhdpi")
    fun settings() = snap("e1") { SettingsScreen(SettingsUiState(aiOnDevice = true), SettingsActions()) }

    @Test
    fun plus() = snap("e2") { PlusScreen(PlusUiState()) }

    @Test
    fun identity() = snap("e3") { IdentityLinkScreen(IdentityLinkUiState()) }

    @Test
    fun delete() = snap("e4") {
        androidx.compose.foundation.layout.Box(contentAlignment = androidx.compose.ui.Alignment.Center) {
            SettingsScreen(SettingsUiState(), SettingsActions())
            app.witness.core.designsystem.Scrim({})
            DeleteAccountCard(running = false, onCancel = {}, onConfirm = {}, modifier = androidx.compose.ui.Modifier.padding(horizontal = 24.dp))
        }
    }

    @Test
    fun privacy() = snap("e5") { PrivacyDataScreen(PrivacyDataUiState()) }

    @Test
    fun privacyWithPublicItems() = snap("e5-public-items") {
        val items = PublicItems.of(listOf(capture(1, at = 1, place = "Denver area"), capture(2, at = 2, label = "Front door"), capture(3, parent = 1)))
        PrivacyDataScreen(PrivacyDataUiState(items = items, confirmItem = items.first()))
    }
}
