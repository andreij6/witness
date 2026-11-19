package app.witness.core.domain.usecase

import app.witness.core.domain.fake.FakeTelemetry
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.model.RecordId
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class TelemetryGateTest {
    @Test
    fun offByDefaultDropsEverything() {
        val sink = FakeTelemetry()
        val gate = TelemetryGate(sink)
        gate.event(AnalyticsEvent.CAPTURE_SEALED)
        gate.breadcrumb("x")
        gate.nonFatal(IllegalStateException("x"))
        assertFalse(gate.isEnabled)
        assertTrue(sink.events.isEmpty() && sink.breadcrumbs.isEmpty() && sink.errors.isEmpty())
    }

    @Test
    fun followsSettingsOptIn() = runTest {
        val sink = FakeTelemetry()
        val gate = TelemetryGate(sink)
        val settings = InMemorySettingsStore()
        val job = gate.follow(settings, backgroundScope)
        runCurrent()
        assertEquals(false, sink.collectionEnabled)
        settings.update { it.copy(telemetryOptIn = true) }
        runCurrent()
        assertTrue(gate.isEnabled)
        assertEquals(true, sink.collectionEnabled)
        gate.event(AnalyticsEvent.CHECK_COMPLETED, mapOf("verdict" to 1L))
        assertEquals(1, sink.events.size)
        settings.update { it.copy(telemetryOptIn = false) }
        runCurrent()
        gate.event(AnalyticsEvent.CHECK_COMPLETED)
        assertEquals(1, sink.events.size)
        assertEquals(false, sink.collectionEnabled)
        job.cancel()
    }

    @Test
    fun metricKeysMustBeSnakeCaseIdentifiers() {
        val sink = FakeTelemetry()
        val gate = TelemetryGate(sink).apply { setCollectionEnabled(true) }
        gate.event(AnalyticsEvent.ANCHOR_RUN, mapOf("anchored" to 1, "WERC8GMRZGE196QV" to 2, "a b" to 3, "" to 4, "x".repeat(41) to 5, "ok_2" to 6))
        assertEquals(mapOf("anchored" to 1L, "ok_2" to 6L), sink.events.single().second)
    }

    @Test
    fun breadcrumbsAndErrorsAreRedacted() {
        val sink = FakeTelemetry()
        val gate = TelemetryGate(sink).apply { setCollectionEnabled(true) }
        gate.breadcrumb("opened WERC-8GMR-ZGE1-96QV at 39.7392,-104.9903")
        gate.nonFatal(IllegalArgumentException("record WERC8GMRZGE196QV failed for a@b.co"))
        assertEquals("opened [redacted] at [redacted],[redacted]", sink.breadcrumbs.single())
        val e = assertIs<RedactedError>(sink.errors.single())
        assertEquals("IllegalArgumentException", e.type)
        assertEquals("record [redacted] failed for [redacted]", e.message)
    }

    @Test
    fun recordIdsWithoutDigitsAreRedacted() {
        val letters = RecordId.fromBytes(ByteArray(10) { 0x55 })
        assertTrue(letters.text.none(Char::isDigit))
        assertEquals("record [redacted] failed", Redactor.scrub("record ${letters.text} failed"))
        assertEquals("[redacted].", Redactor.scrub("${letters.text}."))
        assertEquals("NOTANIDENTIFIER internationalization", Redactor.scrub("NOTANIDENTIFIER internationalization"))
    }

    @Test
    fun redactorKeepsOrdinaryText() {
        assertEquals("anchoring failed after 3 attempts", Redactor.scrub("anchoring failed after 3 attempts"))
        assertEquals("internationalization", Redactor.scrub("internationalization"))
        assertEquals("see [redacted] now", Redactor.scrub("see https://witness.app/p/WERC8GMRZGE196QV now"))
        assertEquals("hash [redacted]", Redactor.scrub("hash e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))
        assertEquals("k [redacted]", Redactor.scrub("k AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"))
        assertEquals("", Redactor.scrub(""))
    }

    @Test
    fun noneTelemetryIsInert() {
        app.witness.core.domain.port.Telemetry.None.apply {
            setCollectionEnabled(true)
            event(AnalyticsEvent.KEY_ROTATED)
            breadcrumb("x")
            nonFatal(Exception())
        }
    }
}
