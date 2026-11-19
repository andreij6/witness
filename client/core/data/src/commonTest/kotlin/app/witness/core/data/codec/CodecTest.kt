package app.witness.core.data.codec

import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.Edit
import app.witness.core.model.ImageFormat
import app.witness.core.model.LocationOpening
import app.witness.core.model.Rect
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class BytesTest {
    @Test
    fun roundTripsAllTypes() {
        val bytes = ByteWriter(1).byte(255).int(-2).long(Long.MIN_VALUE).bytes(byteArrayOf(1, 2)).string("héllo").nullableLong(null).nullableLong(7).toByteArray()
        val r = ByteReader(bytes)
        assertEquals(255, r.byte())
        assertEquals(-2, r.int())
        assertEquals(Long.MIN_VALUE, r.long())
        assertContentEquals(byteArrayOf(1, 2), r.bytes())
        assertEquals("héllo", r.string())
        assertNull(r.nullableLong())
        assertEquals(7, r.nullableLong())
        assertTrue(r.exhausted)
    }

    @Test
    fun growsFromTinyCapacity() {
        val w = ByteWriter(1)
        repeat(1000) { w.int(it) }
        assertEquals(4000, w.toByteArray().size)
    }

    @Test
    fun truncatedInputRejected() {
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteArray(3)).int() }
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteWriter().int(10).toByteArray()).bytes() }
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteArray(0)).byte() }
    }

    @Test
    fun hostileLengthsRejected() {
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteWriter().int(-1).toByteArray()).bytes() }
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteWriter().int(Int.MAX_VALUE).toByteArray()).bytes() }
        assertFailsWith<IllegalArgumentException> { ByteReader(ByteWriter().string("toolong").toByteArray()).string(3) }
    }

    @Test
    fun badOptionalTagRejected() {
        assertFailsWith<IllegalArgumentException> { ByteReader(byteArrayOf(2)).nullableLong() }
    }

    @Test
    fun invalidUtf8Rejected() {
        assertFailsWith<CharacterCodingException> { ByteReader(ByteWriter().bytes(byteArrayOf(0xff.toByte())).toByteArray()).string() }
    }
}

class EditCodecTest {
    private val all = listOf(
        Edit.Resize(1080, 1350, 2),
        Edit.Recompress(ImageFormat.WEBP, 85),
        Edit.Orient(6),
        Edit.Crop(Rect(10, 20, 300, 400)),
        Edit.Rotate(-250),
        Edit.Tone(-100, 50, 32767),
        Edit.Filter(3, 80),
        Edit.Blur(Rect(1, 2, 3, 4), BlurClass.PLATE, 1200),
    )

    @Test
    fun roundTripsEveryKind() = assertEquals(all, EditCodec.decode(EditCodec.encode(all)))

    @Test
    fun roundTripsEmpty() = assertEquals(emptyList(), EditCodec.decode(EditCodec.encode(emptyList())))

    @Test
    fun deterministic() = assertContentEquals(EditCodec.encode(all), EditCodec.encode(all.toList()))

    @Test
    fun tooManyRejected() {
        assertFailsWith<IllegalArgumentException> { EditCodec.encode(List(Edit.MAX_EDITS + 1) { Edit.Orient(1) }) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(byteArrayOf(1, (Edit.MAX_EDITS + 1).toByte())) }
    }

    @Test
    fun malformedRejected() {
        val good = EditCodec.encode(all)
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(byteArrayOf()) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(good.copyOf().also { it[0] = 9 }) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(good.copyOf(good.size - 1)) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(good + 0) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(byteArrayOf(1, 1, 99)) }
        assertFailsWith<IllegalArgumentException> { EditCodec.decode(byteArrayOf(1, 1, 1, 99, 0, 0, 0, 1)) }
    }
}

class ShareCodecTest {
    private val id = CaseId.of(3, ByteArray(15) { it.toByte() })
    private val shares = listOf(
        CaseShare(id, "https://case.witness.app/v/${id.text}#k=abc", 10, 20, 4),
        CaseShare(id, "https://case.witness.app/v/x", 11, null, 0),
    )

    @Test
    fun roundTrips() = assertEquals(shares, ShareCodec.decode(ShareCodec.encode(shares)))

    @Test
    fun emptyIsNull() {
        assertNull(ShareCodec.encode(emptyList()))
        assertEquals(emptyList(), ShareCodec.decode(null))
    }

    @Test
    fun malformedRejected() {
        val good = ShareCodec.encode(shares)!!
        assertFailsWith<IllegalArgumentException> { ShareCodec.decode(good.copyOf().also { it[0] = 2 }) }
        assertFailsWith<IllegalArgumentException> { ShareCodec.decode(good + 1) }
        assertFailsWith<IllegalArgumentException> { ShareCodec.decode(ByteWriter().byte(1).int(-1).toByteArray()) }
        assertFailsWith<IllegalArgumentException> { ShareCodec.decode(ByteWriter().byte(1).int(ShareCodec.MAX_SHARES + 1).toByteArray()) }
    }

    @Test
    fun tooManyOnEncodeRejected() {
        assertFailsWith<IllegalArgumentException> { ShareCodec.encode(List(ShareCodec.MAX_SHARES + 1) { shares[0] }) }
    }
}

class OpeningCodecTest {
    private val salt = ByteString.of(ByteArray(32) { it.toByte() })

    @Test
    fun roundTrips() {
        listOf(
            LocationOpening(salt, 488_583_700, 22_944_810, 350),
            LocationOpening(salt, -LocationOpening.LAT_E7_MAX, -LocationOpening.LON_E7_MAX, 0),
            LocationOpening(salt, LocationOpening.LAT_E7_MAX, LocationOpening.LON_E7_MAX, Int.MAX_VALUE),
        ).forEach { assertEquals(it, OpeningCodec.decode(OpeningCodec.encode(it))) }
    }

    @Test
    fun layoutIsFixedBigEndian() {
        val b = OpeningCodec.encode(LocationOpening(salt, 0x01020304, -1, 0x0A0B0C0D))
        assertEquals(OpeningCodec.SIZE, b.size)
        assertEquals(1, b[0].toInt())
        assertContentEquals(byteArrayOf(1, 2, 3, 4, -1, -1, -1, -1, 10, 11, 12, 13), b.copyOfRange(33, 45))
    }

    @Test
    fun rejectsInvalid() {
        assertFailsWith<IllegalArgumentException> { OpeningCodec.encode(LocationOpening(ByteString.of(ByteArray(31)), 0, 0, 0)) }
        assertFailsWith<IllegalArgumentException> { OpeningCodec.encode(LocationOpening(salt, LocationOpening.LAT_E7_MAX + 1, 0, 0)) }
        assertFailsWith<IllegalArgumentException> { OpeningCodec.encode(LocationOpening(salt, 0, 0, -1)) }
        assertFailsWith<IllegalArgumentException> { OpeningCodec.decode(ByteArray(10)) }
        val good = OpeningCodec.encode(LocationOpening(salt, 0, 0, 0))
        assertFailsWith<IllegalArgumentException> { OpeningCodec.decode(good.copyOf().also { it[0] = 2 }) }
        assertFailsWith<IllegalArgumentException> { OpeningCodec.decode(good.copyOf().also { it[33] = 0x7f }) }
    }
}
