package app.witness.feature.capture

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.CaptureMode
import app.witness.core.model.Dims
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.ui.LocationConfidence
import app.witness.core.ui.pillText
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
class CameraSnapshotTest {
    private fun image(name: String): Photo? = File("../../../design/images/$name.png").takeIf { it.exists() }?.let { Photo(it.readBytes()) }

    private fun snap(name: String, content: @Composable () -> Unit) {
        val controller = Robolectric.buildActivity(ComponentActivity::class.java)
        controller.get().setTheme(android.R.style.Theme_Material_Light_NoActionBar)
        val activity = controller.setup().get()
        activity.setContent { WitnessTheme { content() } }
        repeat(4) {
            ShadowLooper.idleMainLooper()
            Thread.sleep(150)
        }
        ShadowLooper.idleMainLooper()
        val root: View = activity.window.decorView
        val bitmap = Bitmap.createBitmap(root.width, root.height, Bitmap.Config.ARGB_8888)
        root.draw(Canvas(bitmap))
        val dir = File(System.getProperty("snapshotDir") ?: "build/snapshots").apply { mkdirs() }
        File(dir, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        assertTrue(root.width > 0 && root.height > 0)
    }

    private val id = RecordId.fromBytes(ByteArray(10) { it.toByte() })
    private val high = LocationConfidence.High
    private val base = CameraUiState(
        seal = CameraCopy.sealPill(SecurityLevel.STRONG_BOX),
        location = LocationPillUi(high.pillText(), high.icon, high.tone),
        meta = "4:12:09 PM · spoof check passed",
    )

    @Test
    fun cameraPost() {
        val frame = FrameDetections(
            Dims(896, 1200),
            listOf(BlurProposal(Rect(624, 450, 43, 73), BlurClass.HOUSE_NO), BlurProposal(Rect(475, 668, 66, 43), BlurClass.PLATE)),
        )
        val detections = CameraCopy.detections(frame, CaptureMode.POST)
        val state = base.copy(
            frameAspect = 896f / 1200f,
            detections = detections,
            hint = CameraCopy.hint(detections.size, CaptureMode.POST, LocationPrecision.AREA),
            last = LastCapture(id, WitnessState.Anchored, image("rainbow")),
        )
        val view = image("hail")
        snap("b1") { CameraScreen(state, CameraActions.None, {}, {}) { m -> PhotoImage(view, null, m.fillMaxSize(), maxDimension = 1200) } }
    }

    @Test
    fun cameraSealedOffline() {
        val state = base.copy(
            online = false,
            meta = "4:31:52 PM · no network · sealed on-device",
            last = LastCapture(id, WitnessState.Queued, image("hail")),
            toast = SealedToast(id, "Sealed · StrongBox", "Queued for anchoring", Tone.Warn, WitnessIcon.HourglassTop, image("hail")),
        )
        val view = image("rainbow")
        snap("b2") { CameraScreen(state, CameraActions.None, {}, {}) { m -> PhotoImage(view, null, Modifier.fillMaxSize().then(m), maxDimension = 1200) } }
    }

    @Test
    fun caseToastStacksAboveTheCoach() {
        val state = base.copy(
            mode = CaptureMode.CASE,
            last = LastCapture(id, WitnessState.Anchored, image("hail")),
            toast = SealedToast(id, "Sealed · StrongBox", "Anchoring…", Tone.Warn, WitnessIcon.HourglassTop, image("hail")),
        )
        val view = image("rainbow")
        snap("c1-toast") {
            CameraScreen(
                state,
                CameraActions.None,
                {},
                {},
                topAccessory = { Box(Modifier.size(220.dp, 40.dp).background(Color(0xCC6B4BD6))) },
                bottomAccessory = { Box(Modifier.fillMaxWidth().height(110.dp).background(Color(0xE6101216)).border(1.dp, Color(0xFF8B5CF6))) },
            ) { m -> PhotoImage(view, null, Modifier.fillMaxSize().then(m), maxDimension = 1200) }
        }
    }

    @Test
    fun cameraError() = snap("b1-error") { CameraScreen(base.copy(error = CameraCopy.NO_KEY), CameraActions.None, {}, {}) }
}
