package app.witness.core.domain.usecase

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.EraseOutcome
import app.witness.core.domain.port.EraseRequest
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.RecordId
import kotlinx.coroutines.flow.first

data class EraseReport(
    val erased: List<RecordId> = emptyList(),
    val rejected: List<RecordId> = emptyList(),
    val missingKey: List<RecordId> = emptyList(),
    val offline: Boolean = false,
) {
    val complete: Boolean get() = rejected.isEmpty() && missingKey.isEmpty() && !offline
}

class EraseMine(
    private val captures: CaptureStore,
    private val registry: RegistryGateway,
    private val vault: KeyVault,
    private val random: RandomSource,
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend fun publicCopiesOf(recordId: RecordId): EraseReport {
        val root = captures.get(recordId) ?: return EraseReport()
        return erase(listOf(root) + captures.derivativesOf(recordId))
    }

    suspend fun everything(): EraseReport = erase(captures.observeAll().first())

    private suspend fun erase(targets: List<Capture>): EraseReport {
        val erased = ArrayList<RecordId>()
        val rejected = ArrayList<RecordId>()
        val missing = ArrayList<RecordId>()
        for (c in targets) {
            if (!isErasable(c)) continue
            val keyId = c.keyRef!!.keyId
            if (vault.key(keyId) == null) {
                missing += c.recordId
                continue
            }
            val nonce = random.nextBytes(NONCE_SIZE)
            val signature = vault.sign(keyId, message(c.recordId, nonce))
            val outcome = try {
                registry.eraseMine(EraseRequest(c.recordId, keyId, ByteString.of(nonce), signature))
            } catch (_: OfflineException) {
                return EraseReport(erased, rejected, missing, offline = true).also(::report)
            }
            when (outcome) {
                EraseOutcome.Erased -> {
                    captures.update(c.copy(publicErased = true))
                    erased += c.recordId
                }
                is EraseOutcome.Rejected -> rejected += c.recordId
            }
        }
        return EraseReport(erased, rejected, missing).also(::report)
    }

    private fun report(r: EraseReport) {
        telemetry.event(AnalyticsEvent.ERASE_REQUESTED, mapOf("erased" to r.erased.size.toLong(), "rejected" to r.rejected.size.toLong()))
    }

    companion object {
        const val NONCE_SIZE: Int = 16
        val DOMAIN: ByteArray = "witness/erase/v1".encodeToByteArray()

        fun isErasable(c: Capture): Boolean =
            c.isVerified && !c.publicErased && c.status == AnchoringStatus.ANCHORED && c.keyRef != null

        fun message(recordId: RecordId, nonce: ByteArray): ByteArray = DOMAIN + recordId.toBytes() + nonce
    }
}
