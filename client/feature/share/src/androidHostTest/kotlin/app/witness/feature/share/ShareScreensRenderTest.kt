package app.witness.feature.share

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry
import kotlin.test.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w412dp-h915dp-xxhdpi")
class ShareScreensRenderTest {
    private val tints = listOf(Color(0xFF6B7A8F), Color(0xFF8A6F5A), Color(0xFF9AA3A8), Color(0xFF5E7D63))
    private val image: RecordImage = { id -> Box(Modifier.fillMaxSize().background(tints[id.toBytes()[0].toInt() and 3])) }
    private fun rid(n: Int) = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (n + it).toByte() })
    private val url = "https://case.witness.app/v/8F2KQM7DABCDEFGHJKMNPQRSTV#k=c2VjcmV0LWtleS1ieXRlcy1oZXJlLTMyYnl0ZXM"

    private fun state(link: Boolean) = ShareCaseUiState(
        loading = false,
        subtitle = "Fender bender · 4 items",
        items = (0..3).map { ShareItemUi(rid(it), true, it == 3) },
        selectedLabel = "4 of 4 selected",
        privacy = listOf(
            PrivacyChipUi(PrivacyOption.BLUR_FACES, "Blur faces · 1", true),
            PrivacyChipUi(PrivacyOption.BLUR_PLATES, "Blur plates", false),
            PrivacyChipUi(PrivacyOption.HIDE_LOCATION, "Hide location", false),
        ),
        expiry = ShareExpiry.WEEK,
        link = if (link) ShareLinkUi(url, ShareText.displayLink(url), "Expires Nov 25 · 4 items", false) else null,
        recipients = "4 of 4 verified · 1 declared edit",
    )

    private val actions = ShareCaseActions({}, {}, {}, {}, {}, {}, {}, {}, {}, {})

    private fun check(name: String, content: @androidx.compose.runtime.Composable () -> Unit) =
        assertTrue(Snapshot.distinctColors(Snapshot.render(name, content)) > 8, "$name rendered blank")

    @Test
    fun c9ShareCase() = check("c9-share-case") { ShareCaseScreen(state(true), actions, image = image) }

    @Test
    fun c9BeforeLink() = check("c9-share-case-pending") { ShareCaseScreen(state(false), actions, image = image) }

    @Test
    fun c8Export() = check("c8-export") {
        Box(Modifier.fillMaxSize().background(Color(0xFFF4F3EF))) {
            WitnessSheet(true, {}) { ExportContent(ExportUiState(loading = false, subtitle = "Fender bender · 4 items"), {}, {}, {}) }
        }
    }
}
