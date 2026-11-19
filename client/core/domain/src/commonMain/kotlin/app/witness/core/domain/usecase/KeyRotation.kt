package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.DeviceKey
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel

data class RotationStatus(
    val keyId: KeyId,
    val security: SecurityLevel,
    val rotatesInDays: Int,
    val rotationDue: Boolean,
    val windowEndsAtMs: Long,
    val windowExpired: Boolean,
)

sealed interface RotationOutcome {
    data object NotDue : RotationOutcome

    data object NoKey : RotationOutcome

    data object Offline : RotationOutcome

    data class Rotated(val retired: KeyId, val key: DeviceKey) : RotationOutcome

    data class Failed(val reason: String) : RotationOutcome
}

class KeyRotation(
    private val vault: KeyVault,
    private val enrollment: Enrollment,
    private val clock: Clock,
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend fun status(): RotationStatus? = vault.activeKey()?.takeIf { it.isEnrolled && !it.retired }?.let { status(it, clock.nowEpochMillis()) }

    suspend operator fun invoke(force: Boolean = false): RotationOutcome {
        val key = vault.activeKey()?.takeIf { it.isEnrolled && !it.retired } ?: return RotationOutcome.NoKey
        if (!force && !status(key, clock.nowEpochMillis()).rotationDue) return RotationOutcome.NotDue
        return when (val r = enrollment.enrollNew(allowLate = false)) {
            is EnrollmentStatus.Enrolled -> {
                vault.retire(key.keyId)
                telemetry.event(AnalyticsEvent.KEY_ROTATED, mapOf("age_days" to ((clock.nowEpochMillis() - key.enrolledAtMs!!) / DAY_MS)))
                RotationOutcome.Rotated(key.keyId, r.key)
            }
            EnrollmentStatus.Offline, is EnrollmentStatus.Pending -> RotationOutcome.Offline
            is EnrollmentStatus.Unsupported -> RotationOutcome.Failed(r.reason)
        }
    }

    companion object {
        const val DAY_MS: Long = 86_400_000
        const val ROTATE_AFTER_MS: Long = 30 * DAY_MS
        const val SERVER_WINDOW_MS: Long = 60 * DAY_MS

        fun status(key: DeviceKey, nowMs: Long): RotationStatus {
            val enrolled = requireNotNull(key.enrolledAtMs) { "key not enrolled" }
            val rotateAt = enrolled + ROTATE_AFTER_MS
            val windowEnd = key.activeUntilMs ?: (enrolled + SERVER_WINDOW_MS)
            val remaining = rotateAt - nowMs
            return RotationStatus(
                keyId = key.keyId,
                security = key.security,
                rotatesInDays = if (remaining <= 0) 0 else ((remaining + DAY_MS - 1) / DAY_MS).toInt(),
                rotationDue = nowMs >= rotateAt,
                windowEndsAtMs = windowEnd,
                windowExpired = nowMs > windowEnd,
            )
        }
    }
}
