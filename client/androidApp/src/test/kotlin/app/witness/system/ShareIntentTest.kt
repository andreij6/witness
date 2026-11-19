package app.witness.system

import android.app.Application
import android.content.Intent
import android.net.Uri
import app.witness.core.model.RecordId
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class ShareIntentTest {
    private val uri = Uri.parse("content://app.witness.files/shared/x.jpg")

    @Test
    fun postShareCarriesTheImageAndTheCaptionWithItsLink() {
        val i = SystemActions.sendStream(uri, SystemActions.JPEG, "Hail https://witness.app/p/ABC")
        assertEquals(Intent.ACTION_SEND, i.action)
        assertEquals("image/jpeg", i.type)
        assertEquals(uri, i.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java))
        assertEquals("Hail https://witness.app/p/ABC", i.getStringExtra(Intent.EXTRA_TEXT))
        assertTrue(i.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION != 0)
    }

    @Test
    fun fileShareWithoutTextHasNoTextExtra() {
        val i = SystemActions.sendStream(uri, "application/zip", null)
        assertNull(i.getStringExtra(Intent.EXTRA_TEXT))
        assertFalse(i.hasExtra(Intent.EXTRA_TEXT))
    }

    @Test
    fun postFileNameIsSafeAndNamedAfterTheRecord() {
        val record = RecordId.fromDigest(ByteArray(32) { it.toByte() })
        val name = SystemActions.postFileName(record)
        assertEquals("witness-${record.text}.jpg", name)
        assertEquals(name, SystemActions.safeName(name))
    }
}
