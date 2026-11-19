package app.witness.di

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.usecase.TelemetryGate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame

class LocalLogTelemetryTest {
    private class Recording : Telemetry {
        val calls = mutableListOf<String>()
        override fun setCollectionEnabled(enabled: Boolean) { calls += "enabled=$enabled" }
        override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) { calls += "event=${event.name}" }
        override fun breadcrumb(message: String) { calls += "crumb=$message" }
        override fun nonFatal(error: Throwable) { calls += "error=${error.message}" }
    }

    @Test
    fun logsLocallyAndStillDelegatesEverything() {
        val inner = Recording()
        val logged = mutableListOf<Pair<String, Throwable?>>()
        val t = LocalLogTelemetry(inner) { m, e -> logged += m to e }
        val boom = IllegalStateException("boom")
        t.setCollectionEnabled(true)
        t.event(AnalyticsEvent.entries.first())
        t.breadcrumb("step")
        t.nonFatal(boom)
        assertEquals(listOf("enabled=true", "event=${AnalyticsEvent.entries.first().name}", "crumb=step", "error=boom"), inner.calls)
        assertEquals(listOf<Pair<String, Throwable?>>("step" to null, "non-fatal: boom" to boom), logged)
    }

    @Test
    fun theConsentGateIsFoundThroughTheWrapper() {
        val gate = TelemetryGate(Recording())
        assertSame(gate, LocalLogTelemetry.gateOf(gate))
        assertSame(gate, LocalLogTelemetry.gateOf(LocalLogTelemetry.wrapIf(true, gate)))
        assertNull(LocalLogTelemetry.gateOf(LocalLogTelemetry.wrapIf(true, Recording())))
        assertNull(LocalLogTelemetry.gateOf(Telemetry.None))
    }

    @Test
    fun wrapsOnlyLocalBuilds() {
        val inner = Recording()
        assertSame(inner, LocalLogTelemetry.wrapIf(false, inner))
        assertEquals(LocalLogTelemetry::class, LocalLogTelemetry.wrapIf(true, inner)::class)
    }
}
