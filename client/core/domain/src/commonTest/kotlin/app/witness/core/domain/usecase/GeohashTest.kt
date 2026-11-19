package app.witness.core.domain.usecase

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class GeohashTest {
    @Test
    fun areaMatchesThePagesCanisterForKnownCells() {
        assertEquals("near 37.8° N, 122.4° W", Geohash.area("9q8yy"))
        assertEquals("near 51.5° N, 0.1° W", Geohash.area("gcpvj"))
        assertEquals("near 33.9° S, 151.2° E", Geohash.area("r3gx2"))
        assertEquals("near 40.7° N, 74.0° W", Geohash.area("dr5rs"))
    }

    @Test
    fun encodeThenCenterRoundTripsWithinTheCell() {
        val hash = Geohash.encode(397_392_000, -1_049_903_000, 5)
        val (lat, lon) = Geohash.centerNano(hash)!!
        assertEquals(39.74, lat / 1e9, 0.03)
        assertEquals(-104.99, lon / 1e9, 0.03)
    }

    @Test
    fun malformedHashesHaveNoArea() {
        assertNull(Geohash.area(""))
        assertNull(Geohash.area("9q8ya"))
        assertNull(Geohash.area("9Q8YY"))
        assertNull(Geohash.area("0123456789bcd"))
    }
}
