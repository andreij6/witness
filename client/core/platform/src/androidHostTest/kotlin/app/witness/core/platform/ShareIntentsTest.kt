package app.witness.core.platform

import android.content.Intent
import android.net.Uri
import app.witness.core.model.RecordId
import app.witness.core.platform.share.ShareInbox
import app.witness.core.platform.share.ShareIntents
import app.witness.core.platform.share.SharedItem
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], manifest = Config.NONE)
class ShareIntentsTest {
    private val id = RecordId.fromBytes(ByteArray(10) { 3 })

    @Test
    fun singleImage() {
        val i = Intent(Intent.ACTION_SEND).setType("image/jpeg").putExtra(Intent.EXTRA_STREAM, Uri.parse("content://media/1"))
        assertEquals(SharedItem.Images(listOf("content://media/1")), ShareIntents.parse(i))
    }

    @Test
    fun multipleImages() {
        val uris = arrayListOf(Uri.parse("content://a"), Uri.parse("content://b"))
        val i = Intent(Intent.ACTION_SEND_MULTIPLE).setType("image/*").putParcelableArrayListExtra(Intent.EXTRA_STREAM, uris)
        assertEquals(SharedItem.Images(listOf("content://a", "content://b")), ShareIntents.parse(i))
    }

    @Test
    fun sharedTextLink() {
        val i = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "https://witness.app/p/${id.text}")
        assertEquals(SharedItem.ProofLink("https://witness.app/p/${id.text}", id), ShareIntents.parse(i))
    }

    @Test
    fun viewDeepLink() {
        val i = Intent(Intent.ACTION_VIEW, Uri.parse("https://witness.app/p/${id.text}"))
        assertEquals(id, (ShareIntents.parse(i) as SharedItem.ProofLink).recordId)
    }

    @Test
    fun unrelatedIntentsIgnored() {
        assertNull(ShareIntents.parse(null))
        assertNull(ShareIntents.parse(Intent(Intent.ACTION_MAIN)))
        assertNull(ShareIntents.parse(Intent(Intent.ACTION_SEND).setType("image/jpeg")))
    }

    @Test
    fun deliverPutsIntoInbox() {
        val inbox = ShareInbox()
        assertFalse(ShareIntents.deliver(Intent(Intent.ACTION_MAIN), inbox))
        assertTrue(ShareIntents.deliver(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, "http://x.y"), inbox))
        assertEquals(SharedItem.Link("http://x.y"), inbox.consume())
    }
}
