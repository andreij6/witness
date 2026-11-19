package app.witness.core.platform.gnss

object CountryBounds {
    private const val MARGIN = 0.5

    private val boxes: Map<String, DoubleArray> = mapOf(
        "us" to doubleArrayOf(24.4, 49.5, -125.0, -66.9, 51.0, 71.5, -180.0, -129.9, 51.0, 53.0, 172.0, 180.0, 18.9, 22.3, -160.3, -154.8),
        "ca" to doubleArrayOf(41.6, 83.2, -141.1, -52.6),
        "mx" to doubleArrayOf(14.5, 32.8, -118.5, -86.7),
        "gb" to doubleArrayOf(49.8, 60.9, -8.7, 1.8),
        "ie" to doubleArrayOf(51.4, 55.4, -10.7, -5.9),
        "fr" to doubleArrayOf(41.3, 51.1, -5.2, 9.6),
        "de" to doubleArrayOf(47.2, 55.1, 5.8, 15.1),
        "es" to doubleArrayOf(35.9, 43.8, -9.4, 4.4, 27.6, 29.5, -18.2, -13.4),
        "pt" to doubleArrayOf(36.9, 42.2, -9.6, -6.1, 32.6, 33.2, -17.3, -16.2, 36.9, 39.8, -31.3, -25.0),
        "it" to doubleArrayOf(35.4, 47.1, 6.6, 18.6),
        "nl" to doubleArrayOf(50.7, 53.6, 3.3, 7.3),
        "be" to doubleArrayOf(49.5, 51.6, 2.5, 6.5),
        "ch" to doubleArrayOf(45.8, 47.9, 5.9, 10.5),
        "at" to doubleArrayOf(46.3, 49.1, 9.5, 17.2),
        "pl" to doubleArrayOf(49.0, 54.9, 14.1, 24.2),
        "se" to doubleArrayOf(55.3, 69.1, 10.9, 24.2),
        "no" to doubleArrayOf(57.9, 71.2, 4.5, 31.2),
        "ua" to doubleArrayOf(44.3, 52.4, 22.1, 40.3),
        "jp" to doubleArrayOf(24.0, 45.6, 122.9, 146.0),
        "kr" to doubleArrayOf(33.1, 38.7, 124.6, 131.9),
        "cn" to doubleArrayOf(18.1, 53.6, 73.5, 134.8),
        "in" to doubleArrayOf(6.7, 35.7, 68.1, 97.4),
        "au" to doubleArrayOf(-43.7, -10.6, 113.1, 153.7),
        "nz" to doubleArrayOf(-47.3, -34.4, 166.4, 178.6),
        "br" to doubleArrayOf(-33.8, 5.3, -74.0, -34.7),
        "ar" to doubleArrayOf(-55.1, -21.8, -73.6, -53.6),
        "za" to doubleArrayOf(-34.9, -22.1, 16.4, 32.9),
        "ng" to doubleArrayOf(4.2, 13.9, 2.6, 14.7),
        "ke" to doubleArrayOf(-4.7, 5.0, 33.9, 41.9),
    )

    val known: Set<String> get() = boxes.keys

    fun contains(iso: String, lat: Double, lon: Double): Boolean? {
        val b = boxes[iso.lowercase()] ?: return null
        var i = 0
        while (i < b.size) {
            if (lat >= b[i] - MARGIN && lat <= b[i + 1] + MARGIN && lon >= b[i + 2] - MARGIN && lon <= b[i + 3] + MARGIN) return true
            i += 4
        }
        return false
    }
}
