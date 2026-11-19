package app.witness.core.model

enum class Tier { FREE, PLUS }

data class Entitlement(val tier: Tier, val quotaBytes: Long, val expiresAtMs: Long?) {
    fun isActive(nowMs: Long): Boolean = tier == Tier.FREE || expiresAtMs == null || nowMs < expiresAtMs

    fun effectiveTier(nowMs: Long): Tier = if (isActive(nowMs)) tier else Tier.FREE

    companion object {
        const val FREE_CASE_BYTES: Long = 16L * 1024 * 1024
        const val PLUS_QUOTA_BYTES: Long = 25L * 1024 * 1024 * 1024
        const val PLUS_CASE_BYTES: Long = 256L * 1024 * 1024
        val FREE: Entitlement = Entitlement(Tier.FREE, FREE_CASE_BYTES, null)
    }
}

data class Settings(
    val telemetryOptIn: Boolean = false,
    val aiAssistant: Boolean = true,
    val defaultPrecision: LocationPrecision = LocationPrecision.AREA,
    val captureMode: CaptureMode = CaptureMode.POST,
    val activeCase: LocalCaseId? = null,
    val installNonce: ByteString? = null,
)
