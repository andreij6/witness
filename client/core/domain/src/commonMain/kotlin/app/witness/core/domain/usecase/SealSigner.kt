package app.witness.core.domain.usecase

import app.witness.core.domain.port.CounterStore
import app.witness.core.domain.port.PreparedCapture
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SealedCapture
import app.witness.core.model.DeviceKey
import app.witness.core.model.KeyRef
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class SealSigner(private val engine: SealEngine, private val counters: CounterStore) {
    private val section = Mutex()

    suspend fun seal(prepared: PreparedCapture, key: DeviceKey): SealedCapture = section.withLock {
        val counter = counters.reserve(key.keyId)
        engine.seal(prepared, prepared.draft.finalize(KeyRef(key.keyId, counter), key.certSerial))
    }
}
