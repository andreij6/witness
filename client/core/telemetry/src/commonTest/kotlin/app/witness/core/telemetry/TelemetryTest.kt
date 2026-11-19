package app.witness.core.telemetry

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.usecase.TelemetryGate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue

class Recording : Telemetry {
    val calls = mutableListOf<String>()
    var enabled: Boolean? = null
    override fun setCollectionEnabled(enabled: Boolean) { this.enabled = enabled }
    override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) { calls += "event:${event.name}:$metrics" }
    override fun breadcrumb(message: String) { calls += "crumb:$message" }
    override fun nonFatal(error: Throwable) { calls += "error" }
}

class SelectTelemetryTest {
    @Test
    fun unconfiguredIsNoneAndBackendNeverBuilt() {
        var built = false
        assertSame(Telemetry.None, selectTelemetry(false) { built = true; Recording() })
        assertFalse(built)
    }

    @Test
    fun configuredIsGatedAndOffByDefault() {
        val sink = Recording()
        val t = selectTelemetry(true) { sink }
        assertIs<TelemetryGate>(t)
        assertFalse(t.isEnabled)
        t.event(AnalyticsEvent.CAPTURE_SEALED)
        t.breadcrumb("x")
        t.nonFatal(RuntimeException())
        assertTrue(sink.calls.isEmpty())
    }

    @Test
    fun optInForwardsAndPropagatesCollectionFlag() {
        val sink = Recording()
        val t = selectTelemetry(true) { sink }
        t.setCollectionEnabled(true)
        assertEquals(true, sink.enabled)
        t.event(AnalyticsEvent.CHECK_COMPLETED, mapOf("ms" to 5))
        assertEquals(listOf("event:CHECK_COMPLETED:{ms=5}"), sink.calls)
        t.setCollectionEnabled(false)
        assertEquals(false, sink.enabled)
        t.event(AnalyticsEvent.CHECK_COMPLETED)
        assertEquals(1, sink.calls.size)
    }

    @Test
    fun noneIsSilent() {
        Telemetry.None.setCollectionEnabled(true)
        Telemetry.None.event(AnalyticsEvent.ANCHOR_RUN)
        Telemetry.None.breadcrumb("b")
        Telemetry.None.nonFatal(RuntimeException())
    }
}

class TelemetryRulesTest {
    @Test
    fun everyEventNameIsFirebaseValid() {
        AnalyticsEvent.entries.forEach { assertTrue(TelemetryRules.isValidName(TelemetryRules.eventName(it)), it.name) }
        assertEquals("capture_sealed", TelemetryRules.eventName(AnalyticsEvent.CAPTURE_SEALED))
    }

    @Test
    fun invalidNamesRejected() {
        listOf("", "1a", "A", "has space", "dash-x", "firebase_x", "google_x", "ga_x", "a".repeat(41)).forEach {
            assertFalse(TelemetryRules.isValidName(it), it)
        }
        assertTrue(TelemetryRules.isValidName("a".repeat(40)))
    }

    @Test
    fun paramsFilteredAndCapped() {
        val metrics = LinkedHashMap<String, Long>()
        metrics["Bad"] = 1
        for (i in 0 until 40) metrics["k$i"] = i.toLong()
        val out = LinkedHashMap<String, Long>()
        TelemetryRules.forEachParam(metrics) { k, v -> out[k] = v }
        assertEquals(TelemetryRules.MAX_PARAMS, out.size)
        assertFalse("Bad" in out)
        assertEquals(0L, out["k0"])
    }

    @Test
    fun emptyParams() {
        var n = 0
        TelemetryRules.forEachParam(emptyMap()) { _, _ -> n++ }
        assertEquals(0, n)
    }
}
