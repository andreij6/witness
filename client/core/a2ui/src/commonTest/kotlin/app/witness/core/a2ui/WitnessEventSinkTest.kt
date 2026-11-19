package app.witness.core.a2ui

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WitnessEventSinkTest {
    private val received = ArrayList<WitnessEvent>()
    private val sink = WitnessEventSink { received += it }

    private fun target(value: Any?) = mapOf("target" to value)

    @Test
    fun mapsCaptureFor() {
        assertTrue(sink.dispatch("report", "capture_for", target("insurance_card")))
        assertEquals(listOf<WitnessEvent>(WitnessEvent.CaptureFor("report", "insurance_card")), received)
    }

    @Test
    fun mapsEdit() {
        sink.dispatch("report", "edit", target("summary"))
        assertEquals(listOf<WitnessEvent>(WitnessEvent.Edit("report", "summary")), received)
    }

    @Test
    fun mapsRemove() {
        sink.dispatch("report", "remove", target("impact_point"))
        assertEquals(listOf<WitnessEvent>(WitnessEvent.Remove("report", "impact_point")), received)
    }

    @Test
    fun mapsFlag() {
        sink.dispatch("s", "flag", target("checklist"))
        assertEquals(listOf<WitnessEvent>(WitnessEvent.Flag("s", "checklist")), received)
    }

    @Test
    fun dropsEventsOutsideTheAllowlist() {
        listOf("open_url", "submit", "share", "delete", "navigate", "").forEach {
            assertFalse(sink.dispatch("s", it, target("x")), it)
        }
        assertTrue(received.isEmpty())
    }

    @Test
    fun allowlistIsCaseAndWhitespaceSensitive() {
        listOf("FLAG", "Edit", " edit", "remove ", "capture-for").forEach { assertFalse(sink.dispatch("s", it, target("x")), it) }
        assertTrue(received.isEmpty())
    }

    @Test
    fun dropsMissingTarget() {
        assertFalse(sink.dispatch("s", "flag", emptyMap()))
        assertTrue(received.isEmpty())
    }

    @Test
    fun dropsNonStringTarget() {
        listOf(1, true, null, listOf("a"), mapOf("a" to "b")).forEach { assertFalse(sink.dispatch("s", "flag", target(it))) }
        assertTrue(received.isEmpty())
    }

    @Test
    fun dropsMalformedTargets() {
        listOf("", " ", "../etc", "Summary", "a b", "x".repeat(65), "javascript:alert(1)").forEach {
            assertNull(WitnessEventSink.parse("s", "flag", target(it)), it)
        }
    }

    @Test
    fun ignoresExtraContextKeys() {
        val event = WitnessEventSink.parse("s", "edit", mapOf("target" to "summary", "value" to "injected"))
        assertEquals(WitnessEvent.Edit("s", "summary"), event)
    }

    @Test
    fun everyMappedEventParses() {
        listOf(WitnessCatalog.CAPTURE_FOR, WitnessCatalog.EDIT, WitnessCatalog.REMOVE, WitnessCatalog.FLAG).forEach {
            assertEquals("t", WitnessEventSink.parse("s", it, target("t"))?.target)
        }
    }

    @Test
    fun dispatchesInOrder() {
        sink.dispatch("s", "flag", target("a"))
        sink.dispatch("s", "nope", target("b"))
        sink.dispatch("s", "edit", target("c"))
        assertEquals(listOf("a", "c"), received.map { it.target })
    }
}
