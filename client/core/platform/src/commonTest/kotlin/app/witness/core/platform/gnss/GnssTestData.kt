package app.witness.core.platform.gnss

const val SEC = 1_000_000_000L

fun sats(vararg cn0: Double, agc: Double? = null, rate: (Int) -> Double = { 100.0 * it }) =
    cn0.mapIndexed { i, c -> SatelliteSample(i + 1, 1, c, rate(i), agc) }

fun epoch(
    t: Int,
    satellites: List<SatelliteSample> = sats(30.0, 35.0, 40.0, 25.0, 45.0),
    fullBias: Long? = -1_300_000_000_000_000_000L,
    bias: Double = 0.0,
    drift: Double? = 10.0,
    discontinuity: Int = 0,
) = GnssEpoch(t * SEC, t * SEC, fullBias?.plus((drift ?: 0.0).toLong() * t), bias, drift, discontinuity, satellites)

fun goodEpochs(n: Int = 5) = List(n) { epoch(it) }
