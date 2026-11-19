package app.witness.core.designsystem

import androidx.compose.ui.graphics.vector.VectorGroup
import androidx.compose.ui.graphics.vector.VectorPath
import androidx.compose.ui.graphics.vector.addPathNodes
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame
import kotlin.test.assertTrue

class WitnessIconTest {
    @Test
    fun everyIconParses() {
        WitnessIcon.entries.forEach { assertTrue(addPathNodes(it.pathData).isNotEmpty(), it.symbolName) }
    }

    @Test
    fun symbolNamesAreUniqueSnakeCase() {
        val names = WitnessIcon.entries.map { it.symbolName }
        assertEquals(names.size, names.toSet().size)
        names.forEach { assertTrue(it.matches(Regex("[a-z0-9_]+")), it) }
    }

    @Test
    fun catalogCoversDesignSet() {
        assertEquals(141, WitnessIcon.entries.size)
        listOf("photo_camera", "lock", "verified", "satellite_alt", "auto_awesome", "key", "verified_user", "share", "link", "face",
            "compare", "flag", "report", "workspace_premium", "help", "error", "hourglass_top", "check_circle", "edit")
            .forEach { name -> assertTrue(WitnessIcon.entries.any { it.symbolName == name }, name) }
    }

    @Test
    fun vectorsUseMaterialSymbolViewport() {
        val v = WitnessIcon.Lock.imageVector
        assertEquals(IconViewport, v.viewportWidth)
        assertEquals(IconViewport, v.viewportHeight)
        val group = v.root.first() as VectorGroup
        assertEquals(IconViewport, group.translationY)
        assertTrue(group.first() is VectorPath)
    }

    @Test
    fun vectorsAreCached() {
        assertSame(WitnessIcon.Share.imageVector, WitnessIcon.Share.imageVector)
    }
}
