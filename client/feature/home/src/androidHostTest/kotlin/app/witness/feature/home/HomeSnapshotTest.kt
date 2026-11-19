package app.witness.feature.home

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.SecurityLevel
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
class HomeSnapshotTest {
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
    fun splash() = snap("a1") { SplashScreen() }

    @Test
    fun welcome() = snap("a2") { WelcomeScreen({}, {}) }

    @Test
    fun permissions() = snap("a3") { PermissionsScreen(PermissionsUiState(), {}, {}) }

    @Test
    fun deviceSetup() = snap("a4") { DeviceSetupScreen(DeviceSetupUiState.of(SetupPhase.Verifying, SecurityLevel.STRONG_BOX), {}, {}) }

    @Test
    fun unsupported() = snap("a5") { UnsupportedScreen({}) }

    @Test
    fun homeEmpty() = snap("a6") { HomeScreen(HomePreviewData.empty, HomeActions()) }

    @Test
    fun home() = snap("a7") { HomeScreen(HomePreviewData.populated, HomeActions()) }

    @Test
    fun anchoring() = snap("a8") { AnchoringScreen(SnapshotData.anchoring, {}, {}, {}) }

    @Test
    fun search() = snap("a9") { SearchScreen(SnapshotData.search, "7MNV", {}, {}, {}, {}, {}, autoFocus = false) }
}
