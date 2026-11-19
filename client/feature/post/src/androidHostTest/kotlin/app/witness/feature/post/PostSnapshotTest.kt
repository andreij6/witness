package app.witness.feature.post

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.LocationPrecision
import app.witness.core.model.PostDraft
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
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
class PostSnapshotTest {
    private val photo = File("../../../design/images/hail.png").takeIf { it.exists() }?.let { Photo(it.readBytes()) }

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

    private val draft = PostDraft(
        parent = RecordId.fromBytes(ByteArray(10)),
        dims = Dims(896, 1200),
        blurs = listOf(BlurProposal(Rect(627, 454, 36, 64), BlurClass.HOUSE_NO), BlurProposal(Rect(481, 675, 54, 30), BlurClass.PLATE)),
        precision = LocationPrecision.AREA,
        coarse = "9xj64",
        altText = "Hailstones piled on a lawn and a dark car parked on the street outside a house.",
        caption = "Golf-ball hail out of nowhere. Shot it, sealed it.",
        aiGenerated = true,
    )
    private val state = ComposerUiState(
        loading = false,
        subtitle = "Sealed 4:12:09 PM · anchoring…",
        draft = draft,
        place = "Denver area",
        photo = photo,
        captionSuggested = true,
        linkText = "witness.app/p/H4IL-7QK",
    )

    @Test
    fun composer() = snap("b3") { ComposerScreen(state, NoActions, {}) }

    @Test
    fun composerWithRemovedBlur() = snap("b3-removed") { ComposerScreen(state.copy(draft = draft.copy(blurs = listOf(draft.blurs[0], draft.blurs[1].copy(enabled = false)))), NoActions, {}) }

    @Test
    fun editPhoto() = snap("b4") {
        ComposerScreen(state.copy(editor = PhotoEditor.withAspect(PhotoEditor.open(draft), CropAspect.PORTRAIT).let { it.copy(crop = it.crop.copy(x = 100, y = 120, w = 700, h = 875)) }), NoActions, {})
    }

    @Test
    fun editTone() = snap("b4-tone") { ComposerScreen(state.copy(editor = PhotoEditor.open(draft).copy(tool = EditTool.TONE, exposureE2 = 20, saturationE2 = -40)), NoActions, {}) }

    @Test
    fun locationSheet() = snap("b5") { ComposerScreen(state.copy(sheet = ComposerSheet.Location(LocationPrecision.AREA)), NoActions, {}) }

    @Test
    fun shared() = snap("b6") {
        PostSharedScreen(
            PostSharedUiState("https://witness.app/p/H4IL7QK0ABCDEFGH", "witness.app/p/H4IL-7QK", "Sealed 4:12:09 PM · anchored 4:12:41 PM", shows = "Denver area"),
            {},
            {},
        )
    }

    private object NoActions : ComposerActions {
        override fun toggleBlur(index: Int) = Unit
        override fun openLocation() = Unit
        override fun selectPrecision(precision: LocationPrecision) = Unit
        override fun confirmLocation() = Unit
        override fun openAltText() = Unit
        override fun editAltText(text: String) = Unit
        override fun confirmAltText() = Unit
        override fun dismissSheet() = Unit
        override fun setCaption(text: String) = Unit
        override fun setProofLink(enabled: Boolean) = Unit
        override fun flagAi() = Unit
        override fun openEditor() = Unit
        override fun editor(transform: (EditorState) -> EditorState) = Unit
        override fun closeEditor() = Unit
        override fun applyEditor() = Unit
        override fun copyLink() = Unit
        override fun share() = Unit
        override fun dismissError() = Unit
    }
}
