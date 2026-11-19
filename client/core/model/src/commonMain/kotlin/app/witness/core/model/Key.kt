package app.witness.core.model

enum class KeyStatus {
    ACTIVE,
    RETIRED,
    SUSPENDED,
    REVOKED;

    val isTrusted: Boolean get() = this == ACTIVE || this == RETIRED
    val isDistrusted: Boolean get() = this == SUSPENDED || this == REVOKED
}

enum class Provisioning { RKP, FACTORY, DEV }

enum class Freshness { FRESH, LATE }

data class StatusChange(val status: KeyStatus, val sinceMs: Long)

class KeyHistory private constructor(val changes: List<StatusChange>) {
    val current: KeyStatus get() = changes.last().status

    fun statusAt(atMs: Long): KeyStatus? = changes.getOrNull(firstAfter(atMs) - 1)?.status

    fun activeAt(atMs: Long): Boolean = statusAt(atMs)?.isTrusted == true

    fun revokedAfter(atMs: Long): Boolean = firstDistrustedAfter(atMs) != null

    fun firstDistrustedAfter(atMs: Long): StatusChange? {
        for (i in firstAfter(atMs) until changes.size) if (changes[i].status.isDistrusted) return changes[i]
        return null
    }

    fun plus(change: StatusChange): KeyHistory? = of(changes + change)

    private fun firstAfter(atMs: Long): Int {
        var lo = 0
        var hi = changes.size
        while (lo < hi) {
            val mid = (lo + hi) ushr 1
            if (changes[mid].sinceMs <= atMs) lo = mid + 1 else hi = mid
        }
        return lo
    }

    override fun equals(other: Any?): Boolean = other is KeyHistory && changes == other.changes

    override fun hashCode(): Int = changes.hashCode()

    override fun toString(): String = "KeyHistory($changes)"

    companion object {
        const val MAX_HISTORY: Int = 64

        fun enrolled(atMs: Long): KeyHistory = KeyHistory(listOf(StatusChange(KeyStatus.ACTIVE, atMs)))

        fun of(changes: List<StatusChange>): KeyHistory? {
            if (changes.isEmpty() || changes.size > MAX_HISTORY) return null
            for (i in 1 until changes.size) {
                val a = changes[i - 1]
                val b = changes[i]
                if (a.sinceMs >= b.sinceMs || a.status == b.status || a.status == KeyStatus.REVOKED) return null
            }
            return KeyHistory(changes.toList())
        }
    }
}

data class KeyInfo(
    val keyId: KeyId,
    val security: SecurityLevel,
    val provisioning: Provisioning,
    val freshness: Freshness,
    val enrolledAtMs: Long,
    val activeUntilMs: Long,
    val history: KeyHistory,
) {
    val status: KeyStatus get() = history.current
    val isDev: Boolean get() = provisioning == Provisioning.DEV || security == SecurityLevel.DEV

    fun acceptsOriginalAt(wallMs: Long): Boolean = wallMs <= activeUntilMs && status.isTrusted
}

data class DeviceKey(
    val keyId: KeyId,
    val security: SecurityLevel,
    val freshness: Freshness,
    val createdAtMs: Long,
    val enrolledAtMs: Long? = null,
    val activeUntilMs: Long? = null,
    val certSerial: String? = null,
    val retired: Boolean = false,
    val boot: BootState = BootState.VERIFIED,
    val deviceLocked: Boolean = true,
) {
    val isEnrolled: Boolean get() = enrolledAtMs != null
}
