package app.witness.core.platform.crypto

object Preimages {
    private val OFFLINE_TAG = "witness-offline".encodeToByteArray()
    private val LOCATION_TAG = "witness/loc/v1".encodeToByteArray()
    const val SALT_SIZE = 32

    fun offlineChallenge(installNonce: ByteArray, wallMs: Long): ByteArray {
        require(installNonce.isNotEmpty()) { "install nonce required" }
        val out = ByteArray(OFFLINE_TAG.size + installNonce.size + 8)
        OFFLINE_TAG.copyInto(out)
        installNonce.copyInto(out, OFFLINE_TAG.size)
        out.putLong(OFFLINE_TAG.size + installNonce.size, wallMs)
        return out
    }

    fun locationCommitment(salt: ByteArray, latE7: Int, lonE7: Int, accuracyCm: Int): ByteArray {
        require(salt.size == SALT_SIZE) { "salt must be $SALT_SIZE bytes" }
        require(accuracyCm >= 0) { "accuracy must be non-negative" }
        val out = ByteArray(LOCATION_TAG.size + SALT_SIZE + 12)
        LOCATION_TAG.copyInto(out)
        salt.copyInto(out, LOCATION_TAG.size)
        var at = LOCATION_TAG.size + SALT_SIZE
        at = out.putInt(at, latE7)
        at = out.putInt(at, lonE7)
        out.putInt(at, accuracyCm)
        return out
    }

    private fun ByteArray.putInt(at: Int, v: Int): Int {
        for (i in 0 until 4) this[at + i] = (v ushr (24 - 8 * i)).toByte()
        return at + 4
    }

    private fun ByteArray.putLong(at: Int, v: Long) {
        for (i in 0 until 8) this[at + i] = (v ushr (56 - 8 * i)).toByte()
    }
}
