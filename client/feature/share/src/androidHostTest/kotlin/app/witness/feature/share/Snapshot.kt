package app.witness.feature.share

import android.graphics.Bitmap
import android.graphics.Canvas
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import app.witness.core.designsystem.WitnessTheme
import java.io.File
import java.time.Duration
import org.robolectric.Robolectric
import org.robolectric.Shadows

object Snapshot {
    fun render(name: String, content: @Composable () -> Unit): Bitmap {
        val controller = Robolectric.buildActivity(ComponentActivity::class.java)
        controller.get().setTheme(android.R.style.Theme_Material_Light_NoActionBar)
        val activity = controller.setup().get()
        activity.setContent { WitnessTheme { content() } }
        val looper = Shadows.shadowOf(Looper.getMainLooper())
        repeat(20) { looper.idleFor(Duration.ofMillis(50)) }
        val root = activity.window.decorView
        val bmp = Bitmap.createBitmap(root.width, root.height, Bitmap.Config.ARGB_8888)
        root.draw(Canvas(bmp))
        File("build/snapshots").mkdirs()
        File("build/snapshots/$name.png").outputStream().use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
        return bmp
    }

    fun distinctColors(bmp: Bitmap): Int {
        val seen = HashSet<Int>()
        for (y in 0 until bmp.height step 7) for (x in 0 until bmp.width step 7) seen += bmp.getPixel(x, y)
        return seen.size
    }
}
