package app.witness.di

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import app.witness.core.a2ui.TemplateStore
import app.witness.core.data.CheckHistoryStore
import app.witness.core.data.CheckRecord
import app.witness.core.domain.Clock
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.model.TemplateSchema
import app.witness.feature.check.RecentCheck
import app.witness.feature.check.RecentChecks
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.conflate
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.TimeZone
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

object SystemClock : Clock {
    override fun nowEpochMillis(): Long = System.currentTimeMillis()
}

object LocalTime {
    fun offsetMs(epochMs: Long): Long = TimeZone.getDefault().getOffset(epochMs).toLong()
}

class SecureRandomSource(private val random: SecureRandom = SecureRandom()) : RandomSource {
    override fun nextBytes(size: Int): ByteArray = ByteArray(size).also(random::nextBytes)
}

object JcaCrypto : Crypto {
    private val digests = ThreadLocal.withInitial { MessageDigest.getInstance("SHA-256") }

    override suspend fun sha256(data: ByteArray, offset: Int, length: Int): ByteArray {
        val d = digests.get()!!
        d.update(data, offset, length)
        return d.digest()
    }

    override suspend fun aesGcmSeal(key: ByteArray, nonce: ByteArray, aad: ByteArray, plaintext: ByteArray, offset: Int, length: Int): ByteArray {
        require(key.size == Crypto.KEY_SIZE) { "key must be ${Crypto.KEY_SIZE} bytes" }
        require(nonce.size == Crypto.NONCE_SIZE) { "nonce must be ${Crypto.NONCE_SIZE} bytes" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(Crypto.TAG_SIZE * 8, nonce))
        if (aad.isNotEmpty()) cipher.updateAAD(aad)
        return cipher.doFinal(plaintext, offset, length)
    }
}

class RoomRecentChecks(private val store: CheckHistoryStore) : RecentChecks {
    override fun observe(limit: Int): Flow<List<RecentCheck>> = store.observeRecent(limit).map { rows ->
        ArrayList<RecentCheck>(rows.size).apply { for (r in rows) add(RecentCheck(r.label.orEmpty(), r.checkedAtMs, r.verdict, r.recordId)) }
    }

    override suspend fun record(check: RecentCheck) = store.record(CheckRecord(check.checkedAtMs, check.verdict, check.recordId, check.label))
}

object Connectivity {
    fun online(context: Context): Flow<Boolean> = callbackFlow {
        val manager = context.getSystemService(ConnectivityManager::class.java)
        if (manager == null) {
            trySend(true)
            awaitClose()
            return@callbackFlow
        }
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) {
                trySend(caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED))
            }

            override fun onLost(network: Network) {
                trySend(false)
            }
        }
        trySend(manager.activeNetwork?.let(manager::getNetworkCapabilities)?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true)
        manager.registerDefaultNetworkCallback(callback)
        awaitClose { manager.unregisterNetworkCallback(callback) }
    }.conflate().distinctUntilChanged()
}

class BundledTemplates(private val load: suspend () -> TemplateStore = { TemplateStore.bundled() }) : TemplateCatalog {
    private val mutex = Mutex()

    @Volatile
    private var store: TemplateStore? = null

    suspend fun store(): TemplateStore = store ?: mutex.withLock { store ?: load().also { store = it } }

    override suspend fun templates(): List<TemplateSchema> = store().templates()
}
