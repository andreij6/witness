package app.witness.core.platform.gnss

import android.annotation.SuppressLint
import android.location.GnssMeasurement
import android.location.GnssMeasurementsEvent
import android.location.Location
import android.location.LocationManager
import android.os.Build
import android.os.CancellationSignal
import android.os.Handler
import android.os.SystemClock
import android.telephony.TelephonyManager
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.LocationReading
import app.witness.core.model.ByteString
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationOpening
import app.witness.core.platform.crypto.Preimages
import app.witness.core.platform.keys.KeystoreKeyVault
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import java.security.SecureRandom
import java.util.concurrent.Executor
import kotlin.coroutines.resume
import kotlin.math.roundToInt
import kotlin.math.roundToLong

class AndroidLocationEvidenceSource(
    private val locationManager: LocationManager,
    private val telephony: TelephonyManager?,
    private val executor: Executor,
    private val handler: Handler,
    private val heuristics: GnssHeuristics = GnssHeuristics(),
    private val random: SecureRandom = SecureRandom(),
) : LocationEvidenceSource {
    private val buffer = GnssRingBuffer()
    private val lock = Any()
    private var registered = false

    private val callback = object : GnssMeasurementsEvent.Callback() {
        override fun onGnssMeasurementsReceived(event: GnssMeasurementsEvent) {
            val epoch = event.toEpoch(SystemClock.elapsedRealtimeNanos())
            synchronized(lock) { buffer.add(epoch) }
        }
    }

    @SuppressLint("MissingPermission")
    fun start(): Boolean {
        if (registered) return true
        registered = runCatching { locationManager.registerGnssMeasurementsCallback(callback, handler) }.getOrDefault(false)
        return registered
    }

    fun stop() {
        if (!registered) return
        locationManager.unregisterGnssMeasurementsCallback(callback)
        registered = false
        synchronized(lock) { buffer.clear() }
    }

    override suspend fun current(): LocationReading? {
        val location = fix() ?: return null
        val epochs = synchronized(lock) { buffer.snapshot(SystemClock.elapsedRealtimeNanos()) }
        val gnss = heuristics.assess(epochs)
        val fix = Fix(
            latE7 = (location.latitude * 1e7).roundToLong().toInt(),
            lonE7 = (location.longitude * 1e7).roundToLong().toInt(),
            accuracyCm = if (location.hasAccuracy()) (location.accuracy * 100).roundToInt().coerceAtLeast(0) else Int.MAX_VALUE,
            isMock = location.isMockCompat(),
        )
        val verdict = LocationAssessor.assess(fix, gnss, telephony?.networkCountryIso)
        val salt = ByteArray(Preimages.SALT_SIZE).also(random::nextBytes)
        val commit = KeystoreKeyVault.sha256(Preimages.locationCommitment(salt, fix.latE7, fix.lonE7, fix.accuracyCm))
        return LocationReading(
            evidence = LocationEvidence(ByteString.wrap(commit), verdict.confidence, verdict.reasons),
            opening = LocationOpening(ByteString.wrap(salt), fix.latE7, fix.lonE7, fix.accuracyCm),
            gnssTimeMs = gnss.gnssTimeMs,
            gnss = verdict.checks,
        )
    }

    @SuppressLint("MissingPermission")
    private suspend fun fix(): Location? {
        val last = runCatching { locationManager.getLastKnownLocation(LocationManager.GPS_PROVIDER) }.getOrNull()
        if (last != null && SystemClock.elapsedRealtimeNanos() - last.elapsedRealtimeNanos <= MAX_FIX_AGE_NS) return last
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return last
        return withTimeoutOrNull(FIX_TIMEOUT_MS) {
            suspendCancellableCoroutine { cont ->
                val signal = CancellationSignal()
                cont.invokeOnCancellation { signal.cancel() }
                runCatching {
                    locationManager.getCurrentLocation(LocationManager.GPS_PROVIDER, signal, executor) { cont.resume(it) }
                }.onFailure { cont.resume(null) }
            }
        } ?: last
    }

    companion object {
        const val MAX_FIX_AGE_NS = 30_000_000_000L
        const val FIX_TIMEOUT_MS = 4_000L
    }
}

@Suppress("DEPRECATION")
internal fun Location.isMockCompat(): Boolean = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) isMock else isFromMockProvider

internal fun GnssMeasurementsEvent.toEpoch(elapsedNs: Long): GnssEpoch {
    val c = clock
    val sats = ArrayList<SatelliteSample>(measurements.size)
    for (m in measurements) {
        sats += SatelliteSample(
            svid = m.svid,
            constellation = m.constellationType,
            cn0DbHz = m.cn0DbHz,
            pseudorangeRateMps = m.pseudorangeRateMetersPerSecond,
            agcDb = if (m.hasAutomaticGainControlLevelDb()) agcOf(m) else null,
        )
    }
    return GnssEpoch(
        elapsedNs = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && c.hasElapsedRealtimeNanos()) c.elapsedRealtimeNanos else elapsedNs,
        timeNanos = c.timeNanos,
        fullBiasNanos = if (c.hasFullBiasNanos()) c.fullBiasNanos else null,
        biasNanos = if (c.hasBiasNanos()) c.biasNanos else 0.0,
        driftNsPerS = if (c.hasDriftNanosPerSecond()) c.driftNanosPerSecond else null,
        discontinuityCount = c.hardwareClockDiscontinuityCount,
        satellites = sats,
    )
}

@Suppress("DEPRECATION")
private fun agcOf(m: GnssMeasurement): Double = m.automaticGainControlLevelDb
