package app.witness.core.platform.share

import app.witness.core.model.RecordId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ShareParserTest {
    private val id = RecordId.fromBytes(ByteArray(10) { it.toByte() })

    @Test
    fun imagesFromStreams() {
        assertEquals(SharedItem.Images(listOf("content://a", "content://b")), ShareParser.parse("image/jpeg", null, listOf("content://a", "content://a", " ", "content://b")))
    }

    @Test
    fun onlyContentUrisAreAcceptedAsImages() {
        val private = listOf("file:///data/data/app.witness/files/captures/x.jpg", "FILE:///sdcard/a.jpg", "/data/data/app.witness/shared_prefs/witness_keys.xml", "android.resource://app.witness/raw/x", "http://evil/x.jpg", "content://")
        assertNull(ShareParser.parse("image/jpeg", null, private))
        assertEquals(SharedItem.Images(listOf("content://media/1", "CONTENT://media/2")), ShareParser.parse("image/jpeg", null, private + "content://media/1" + "CONTENT://media/2"))
        assertEquals(SharedItem.Link("https://witness.app/x"), ShareParser.parse("image/jpeg", "https://witness.app/x", listOf("file:///x.jpg")))
    }

    @Test
    fun imagesCapped() {
        val item = ShareParser.parse("image/*", null, List(50) { "content://$it" }) as SharedItem.Images
        assertEquals(ShareParser.MAX_IMAGES, item.uris.size)
    }

    @Test
    fun proofLinkRecognised() {
        val url = "https://witness.app/p/${id.text}"
        assertEquals(SharedItem.ProofLink(url, id), ShareParser.parse("text/plain", "Look: $url!", emptyList()))
    }

    @Test
    fun proofLinkWithQueryAndGrouped() {
        assertEquals(id, ShareParser.proofRecord("https://witness.app/p/${id.text}?ref=x"))
        assertEquals(id, ShareParser.proofRecord("https://witness.app/p/${id.text.lowercase()}#k"))
    }

    @Test
    fun otherLinksArePlain() {
        assertEquals(SharedItem.Link("http://example.com/x"), ShareParser.parse("text/plain", "see http://example.com/x.", emptyList()))
        assertEquals(SharedItem.Link("https://witness.app/p/notanid"), ShareParser.parse("text/plain", "https://witness.app/p/notanid", emptyList()))
    }

    @Test
    fun nothingUsable() {
        assertNull(ShareParser.parse("text/plain", "no links here", emptyList()))
        assertNull(ShareParser.parse(null, null, emptyList()))
        assertNull(ShareParser.parse("image/png", null, emptyList()))
        assertNull(ShareParser.firstUrl("https://"))
    }

    @Test
    fun firstUrlPicksEarliestScheme() {
        assertEquals("http://a.b", ShareParser.firstUrl("http://a.b then https://c.d"))
        assertEquals("HTTPS://A.B/c", ShareParser.firstUrl("<HTTPS://A.B/c>"))
    }
}

class ShareInboxTest {
    @Test
    fun offerThenConsumeOnce() {
        val inbox = ShareInbox()
        val item = SharedItem.Link("https://x.y")
        inbox.offer(item)
        assertEquals(item, inbox.incoming.value)
        assertEquals(item, inbox.consume())
        assertNull(inbox.consume())
        assertNull(inbox.incoming.value)
    }

    @Test
    fun latestWins() {
        val inbox = ShareInbox()
        inbox.offer(SharedItem.Link("https://a.b"))
        inbox.offer(SharedItem.Link("https://c.d"))
        assertEquals(SharedItem.Link("https://c.d"), inbox.consume())
    }
}
