package app.witness.nativelib

import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.OfflineException
import app.witness.core.model.ByteString
import app.witness.core.model.KeyId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import uniffi.witness_android.KeystoreSigner
import uniffi.witness_android.WitnessException
import uniffi.witness_android.WitnessInterface
import java.io.File
import java.math.BigInteger
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate

class VaultSigner(private val vault: KeyVault, private val keyId: KeyId, private val chain: List<ByteArray>) : KeystoreSigner {
    override fun sign(message: ByteArray): ByteArray =
        try {
            runBlocking { vault.sign(keyId, message).toByteArray() }
        } catch (e: CancellationException) {
            throw WitnessException.Signer("cancelled")
        } catch (e: Exception) {
            throw WitnessException.Signer(e.message ?: e.javaClass.simpleName)
        }

    override fun certChain(): List<ByteArray> = chain
}

class NativeTrust(private val witness: WitnessInterface, private val file: File, private val pinned: ByteArray? = null) {
    private val lock = Mutex()

    @Volatile
    private var cached: ByteArray? = null

    suspend fun caRoot(): ByteArray = pinned ?: cached ?: lock.withLock {
        cached ?: (stored() ?: fetch()).also { cached = it }
    }

    suspend fun remember(der: ByteArray) = lock.withLock {
        if (cached?.contentEquals(der) != true) {
            persist(der)
            cached = der
        }
    }

    private suspend fun stored(): ByteArray? = withContext(Dispatchers.IO) { file.takeIf(File::isFile)?.readBytes()?.takeIf { it.isNotEmpty() } }

    private suspend fun fetch(): ByteArray =
        (remote { witness.caRoot() } ?: throw OfflineException("registry has no ca root yet")).also { persist(it) }

    private suspend fun persist(der: ByteArray) = withContext(Dispatchers.IO) {
        file.parentFile?.mkdirs()
        val tmp = File(file.path + ".tmp")
        tmp.writeBytes(der)
        check(tmp.renameTo(file)) { "could not persist ca root" }
    }
}

class NativeKeys(
    private val vault: KeyVault,
    private val trust: NativeTrust,
    private val certificate: suspend (KeyId) -> ByteString?,
) {
    suspend fun signer(keyId: KeyId): VaultSigner =
        VaultSigner(vault, keyId, listOfNotNull(certificate(keyId)?.toByteArray(), trust.caRoot()))

    suspend fun caRoot(): ByteArray = trust.caRoot()

    suspend fun activeEnrolled(): KeyId? = vault.activeKey()?.takeIf { it.isEnrolled && !it.retired }?.keyId

    companion object {
        fun serialHex(certificateDer: ByteArray): String {
            val cert = CertificateFactory.getInstance("X.509").generateCertificate(certificateDer.inputStream()) as X509Certificate
            return Hex.of(magnitude(cert.serialNumber))
        }

        fun magnitude(serial: BigInteger): ByteArray {
            val bytes = serial.toByteArray()
            var start = 0
            while (start < bytes.size - 1 && bytes[start] == 0.toByte()) start++
            return bytes.copyOfRange(start, bytes.size)
        }
    }
}
