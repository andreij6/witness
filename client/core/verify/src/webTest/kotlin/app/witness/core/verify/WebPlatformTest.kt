package app.witness.core.verify

import app.witness.core.verify.web.BridgeVerifyEngine
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WebPlatformTest {
    private fun bmp(w: Int, h: Int): ByteArray {
        val row = (w * 3 + 3) and 3.inv()
        val size = 54 + row * h
        fun le32(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte(), (v ushr 16).toByte(), (v ushr 24).toByte())
        fun le16(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte())
        return "BM".encodeToByteArray() + le32(size) + le32(0) + le32(54) +
            le32(40) + le32(w) + le32(h) + le16(1) + le16(24) + le32(0) + le32(row * h) + le32(2835) + le32(2835) + le32(0) + le32(0) +
            ByteArray(row * h) { 0x7f }
    }

    @Test
    fun fallbackEngineReportsStagesAndNoRecord() = runTest {
        val updates = BridgeVerifyEngine(null).check(byteArrayOf(1, 2, 3)).toList()
        val done = assertIs<CheckUpdate.Done>(updates.last())
        assertEquals(Judgement.NO_RECORD.copy(fallback = true), done.judgement)
        val steps = updates.fold(CheckSteps.INITIAL) { s, u -> s.reduce(u) }
        assertEquals(StageState.DONE, steps[CheckStage.READ_FILE])
        assertTrue(CheckStage.entries.drop(1).all { steps[it] == StageState.SKIPPED })
    }

    @Test
    fun unreachableEngineFallsBack() = runTest {
        val engine = BridgeVerifyEngine("data:text/javascript,export const nothing = 1;")
        assertTrue(engine.isFallback())
        assertTrue(engine.judge(byteArrayOf(1)).fallback)
        assertTrue(BridgeVerifyEngine(null).isFallback())
    }

    @Test
    fun createVerifyEngineUsesBridge() = runTest {
        assertTrue(createVerifyEngine().judge(byteArrayOf(9)).fallback)
    }

    @Test
    fun readerSniffsKnownFormats() = runTest {
        assertEquals(ImageInfo(ImageFormat.PNG, 7, 8, 29), platformImageInfoReader().read(ImageFixtures.png(7, 8)))
    }

    @Test
    fun readerDecodesOtherFormatsInTheBrowser() = runTest {
        val bytes = bmp(5, 3)
        assertEquals(ImageInfo(ImageFormat.UNKNOWN, 5, 3, bytes.size), platformImageInfoReader().read(bytes))
    }

    @Test
    fun readerReturnsNullForGarbage() = runTest {
        assertNull(platformImageInfoReader().read(ByteArray(16) { 3 }))
    }
}
