package app.witness.core.platform.keys

import app.witness.core.model.BootState
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel

object KeyRecordCodec {
    private const val SEP = '|'
    private const val VERSION = "1"
    private const val FIELDS = 11

    fun encode(key: DeviceKey): String {
        key.certSerial?.let { s -> require(s.isNotEmpty() && s.none { it == SEP || it.isWhitespace() }) { "invalid cert serial" } }
        return buildString(96) {
            append(VERSION).append(SEP)
            append(key.keyId.text).append(SEP)
            append(key.security.name).append(SEP)
            append(key.freshness.name).append(SEP)
            append(key.createdAtMs).append(SEP)
            append(key.enrolledAtMs ?: "").append(SEP)
            append(key.activeUntilMs ?: "").append(SEP)
            append(key.certSerial ?: "").append(SEP)
            append(if (key.retired) '1' else '0').append(SEP)
            append(key.boot.name).append(SEP)
            append(if (key.deviceLocked) '1' else '0')
        }
    }

    fun decode(text: String): DeviceKey? {
        val f = text.split(SEP)
        if (f.size != FIELDS || f[0] != VERSION) return null
        val enrolled = if (f[5].isEmpty()) null else f[5].toLongOrNull() ?: return null
        val activeUntil = if (f[6].isEmpty()) null else f[6].toLongOrNull() ?: return null
        return DeviceKey(
            keyId = KeyId.parseOrNull(f[1]) ?: return null,
            security = SecurityLevel.entries.firstOrNull { it.name == f[2] } ?: return null,
            freshness = Freshness.entries.firstOrNull { it.name == f[3] } ?: return null,
            createdAtMs = f[4].toLongOrNull() ?: return null,
            enrolledAtMs = enrolled,
            activeUntilMs = activeUntil,
            certSerial = f[7].ifEmpty { null },
            retired = flag(f[8]) ?: return null,
            boot = BootState.entries.firstOrNull { it.name == f[9] } ?: return null,
            deviceLocked = flag(f[10]) ?: return null,
        )
    }

    private fun flag(s: String): Boolean? = when (s) {
        "1" -> true
        "0" -> false
        else -> null
    }
}
