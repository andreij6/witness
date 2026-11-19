package app.witness.core.a2ui

import app.witness.core.a2ui.ui.ImpactZones
import app.witness.core.a2ui.ui.ZoneRect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ImpactZonesTest {
    @Test
    fun rearDriverIsBottomLeftQuarter() = assertEquals(ZoneRect(0f, 0.75f, 0.5f, 1f), ImpactZones.rect("rear_driver"))

    @Test
    fun frontPassengerIsTopRight() = assertEquals(ZoneRect(0.5f, 0f, 1f, 0.25f), ImpactZones.rect("front_passenger"))

    @Test
    fun sideDriverSpansTheMiddle() = assertEquals(ZoneRect(0f, 0.25f, 0.5f, 0.75f), ImpactZones.rect("side_driver"))

    @Test
    fun centerIsHorizontallyCentered() = assertEquals(ZoneRect(0.25f, 0.75f, 0.75f, 1f), ImpactZones.rect("rear_center"))

    @Test
    fun everyCarTemplateZoneResolves() {
        listOf("front_driver", "front_center", "front_passenger", "side_driver", "side_passenger", "rear_driver", "rear_center", "rear_passenger")
            .forEach { kotlin.test.assertNotNull(ImpactZones.rect(it), it) }
    }

    @Test
    fun unknownZonesHaveNoRect() {
        listOf(null, "", "rear", "_driver", "rear_", "roof_driver", "rear_left", "REAR_DRIVER", "rear_driver_x").forEach {
            assertNull(ImpactZones.rect(it), it.toString())
        }
    }
}
