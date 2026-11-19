package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.GeneratedKey
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.ByteString
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.OutboxState

sealed interface EnrollmentStatus {
    data class Enrolled(val key: DeviceKey) : EnrollmentStatus

    data class Pending(val key: DeviceKey) : EnrollmentStatus

    data class Unsupported(val reason: String) : EnrollmentStatus

    data object Offline : EnrollmentStatus
}

class Enrollment(
    private val vault: KeyVault,
    private val registry: RegistryGateway,
    private val outbox: OutboxStore,
    private val crypto: Crypto,
    private val settings: SettingsStore,
    private val random: RandomSource,
    private val scheduler: AnchorScheduler,
    private val clock: Clock,
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend operator fun invoke(): EnrollmentStatus {
        val current = vault.activeKey()?.takeUnless { it.retired }
        if (current != null && current.isEnrolled) return EnrollmentStatus.Enrolled(current)
        val now = clock.nowEpochMillis()
        if (current != null && now - current.createdAtMs <= LATE_WINDOW_MS) {
            val generated = vault.attestation(current.keyId)
            if (generated != null) return enroll(generated) { EnrollmentStatus.Pending(current) }
        }
        if (current != null) abandon(current.keyId)
        return enrollNew(allowLate = true)
    }

    suspend fun enrollNew(allowLate: Boolean): EnrollmentStatus {
        val online = try {
            registry.challenge()
        } catch (_: OfflineException) {
            null
        }
        if (online == null && !allowLate) return EnrollmentStatus.Offline
        val generated = if (online != null) vault.generate(online, Freshness.FRESH) else vault.generate(offlineChallenge(), Freshness.LATE)
        return enroll(generated) { EnrollmentStatus.Pending(generated.key) }
    }

    private suspend fun enroll(generated: GeneratedKey, offline: () -> EnrollmentStatus): EnrollmentStatus {
        val key = generated.key
        val outcome = try {
            registry.enroll(EnrollmentRequest(key.keyId, generated.attestationChain, key.freshness, generated.challenge))
        } catch (_: OfflineException) {
            return offline()
        }
        return when (outcome) {
            is EnrollOutcome.Enrolled -> {
                vault.markEnrolled(key.keyId, outcome.info.enrolledAtMs, outcome.info.activeUntilMs, outcome.certSerial, outcome.c2paCert)
                scheduler.requestAnchoring()
                telemetry.event(AnalyticsEvent.KEY_ENROLLED, mapOf("fresh" to if (key.freshness == Freshness.FRESH) 1L else 0L))
                EnrollmentStatus.Enrolled(vault.key(key.keyId) ?: key)
            }
            is EnrollOutcome.Rejected -> {
                vault.delete(key.keyId)
                EnrollmentStatus.Unsupported(outcome.reason)
            }
        }
    }

    private suspend fun abandon(keyId: KeyId) {
        vault.retire(keyId)
        for (e in outbox.all()) if (e.keyId == keyId && e.state == OutboxState.AWAITING_ENROLLMENT) outbox.update(e.copy(state = OutboxState.AWAITING_RESEAL))
    }

    private suspend fun offlineChallenge(): ByteString {
        val nonce = settings.update { s -> if (s.installNonce != null) s else s.copy(installNonce = ByteString.of(random.nextBytes(INSTALL_NONCE_SIZE))) }.installNonce!!
        val wall = clock.nowEpochMillis()
        val msg = OFFLINE_DOMAIN + nonce.toByteArray() + ByteArray(8) { i -> (wall ushr (56 - 8 * i)).toByte() }
        return ByteString.of(crypto.sha256(msg))
    }

    companion object {
        const val LATE_WINDOW_MS: Long = 7L * 86_400_000
        const val INSTALL_NONCE_SIZE: Int = 32
        val OFFLINE_DOMAIN: ByteArray = "witness-offline".encodeToByteArray()
    }
}
