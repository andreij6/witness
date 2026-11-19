package app.witness.core.telemetry

import androidx.test.core.app.ApplicationProvider
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], manifest = Config.NONE)
class AndroidTelemetryTest {
    @Test
    fun withoutFirebaseConfigTelemetryIsNoop() {
        assertFalse(AndroidTelemetry.isConfigured(ApplicationProvider.getApplicationContext()))
        assertSame(Telemetry.None, AndroidTelemetry.create(ApplicationProvider.getApplicationContext()))
    }

    @Test
    fun metricsBecomeLongBundle() {
        val bundle = mapOf("ms" to 12L, "Bad Key" to 1L).toBundle()
        assertEquals(1, bundle.size())
        assertEquals(12L, bundle.getLong("ms"))
    }

    @Test
    fun emptyMetricsBecomeEmptyBundle() {
        assertEquals(0, emptyMap<String, Long>().toBundle().size())
    }
}
