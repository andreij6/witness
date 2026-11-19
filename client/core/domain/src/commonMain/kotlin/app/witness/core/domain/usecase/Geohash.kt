package app.witness.core.domain.usecase

import app.witness.core.model.LocationOpening

object Geohash {
    private const val ALPHABET = "0123456789bcdefghjkmnpqrstuvwxyz"
    const val MAX_CHARS: Int = 12

    fun encode(latE7: Int, lonE7: Int, chars: Int): String {
        require(chars in 1..MAX_CHARS) { "geohash length out of range" }
        require(latE7 in -LocationOpening.LAT_E7_MAX..LocationOpening.LAT_E7_MAX) { "latitude out of range" }
        require(lonE7 in -LocationOpening.LON_E7_MAX..LocationOpening.LON_E7_MAX) { "longitude out of range" }
        var latLo = -LocationOpening.LAT_E7_MAX.toLong() shl SCALE
        var latHi = LocationOpening.LAT_E7_MAX.toLong() shl SCALE
        var lonLo = -LocationOpening.LON_E7_MAX.toLong() shl SCALE
        var lonHi = LocationOpening.LON_E7_MAX.toLong() shl SCALE
        val lat = latE7.toLong() shl SCALE
        val lon = lonE7.toLong() shl SCALE
        val out = CharArray(chars)
        var even = true
        for (c in 0 until chars) {
            var idx = 0
            repeat(5) {
                idx = idx shl 1
                if (even) {
                    val mid = (lonLo + lonHi) shr 1
                    if (lon >= mid) { idx = idx or 1; lonLo = mid } else lonHi = mid
                } else {
                    val mid = (latLo + latHi) shr 1
                    if (lat >= mid) { idx = idx or 1; latLo = mid } else latHi = mid
                }
                even = !even
            }
            out[c] = ALPHABET[idx]
        }
        return out.concatToString()
    }

    fun of(opening: LocationOpening, chars: Int): String = encode(opening.latE7, opening.lonE7, chars)

    fun centerNano(hash: String): Pair<Long, Long>? {
        if (hash.isEmpty() || hash.length > MAX_CHARS) return null
        val lat = longArrayOf(-90 * NANO, 90 * NANO)
        val lon = longArrayOf(-180 * NANO, 180 * NANO)
        var even = true
        for (c in hash) {
            val idx = ALPHABET.indexOf(c)
            if (idx < 0) return null
            for (b in 4 downTo 0) {
                val r = if (even) lon else lat
                val mid = (r[0] + r[1]) / 2
                if ((idx shr b) and 1 == 1) r[0] = mid else r[1] = mid
                even = !even
            }
        }
        return (lat[0] + lat[1]) / 2 to (lon[0] + lon[1]) / 2
    }

    fun area(hash: String): String? {
        val (lat, lon) = centerNano(hash) ?: return null
        return "near ${degrees(tenths(lat), 'N', 'S')}, ${degrees(tenths(lon), 'E', 'W')}"
    }

    private fun tenths(v: Long): Long = (v + v.sign * (NANO / 20)) / (NANO / 10)

    private val Long.sign: Long get() = if (this < 0) -1 else if (this > 0) 1 else 0

    private fun degrees(tenths: Long, pos: Char, neg: Char): String {
        val a = kotlin.math.abs(tenths)
        return "${a / 10}.${a % 10}° ${if (tenths < 0) neg else pos}"
    }

    private const val SCALE = 20
    private const val NANO = 1_000_000_000L
}
