package app.witness.core.designsystem.qr

import app.witness.core.designsystem.forEachDarkRun
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class QrPathTest {
    @Test
    fun runsCoverEveryDarkModuleExactlyOnce() {
        val qr = QrEncoder.encode("https://witness.app/p/H4IL-7QK")
        var total = 0
        var runs = 0
        qr.forEachDarkRun { x, y, length ->
            assertTrue(length > 0)
            for (i in x until x + length) assertTrue(qr[i, y])
            if (x > 0) assertTrue(!qr[x - 1, y])
            if (x + length < qr.size) assertTrue(!qr[x + length, y])
            total += length
            runs++
        }
        assertEquals(qr.darkModules(), total)
        assertTrue(runs < total)
    }

    @Test
    fun finderRowIsSingleRunAtEachCorner() {
        val qr = QrEncoder.encode("1")
        val firstRow = mutableListOf<Pair<Int, Int>>()
        qr.forEachDarkRun { x, y, length -> if (y == 0) firstRow += x to length }
        assertEquals(0 to 7, firstRow.first())
        assertEquals(qr.size - 7 to 7, firstRow.last())
    }
}
