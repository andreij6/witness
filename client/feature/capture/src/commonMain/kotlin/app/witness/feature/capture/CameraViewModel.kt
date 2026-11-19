package app.witness.feature.capture

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.LocationReading
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealOutcome
import app.witness.core.model.Capture
import app.witness.core.model.CaptureMode
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Settings
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.mapLatest
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class CameraViewModel(
    private val seal: SealCapture,
    private val captures: CaptureStore,
    private val settings: SettingsStore,
    private val vault: KeyVault,
    private val location: LocationEvidenceSource,
    online: Flow<Boolean>,
    detections: Flow<FrameDetections>,
    ticks: Flow<Long>,
    private val utcOffset: (Long) -> Long = { 0L },
    private val telemetry: Telemetry = Telemetry.None,
) : ViewModel(), CameraActions {
    private data class Local(
        val security: SecurityLevel? = null,
        val reading: LocationReading? = null,
        val toastId: RecordId? = null,
        val sealing: Boolean = false,
        val flash: Flash = Flash.OFF,
        val lens: Lens = Lens.BACK,
        val error: String? = null,
    )

    private data class Recent(val last: Capture?, val toast: Capture?, val lastPhoto: Photo? = null, val toastPhoto: Photo? = null)

    private data class Env(val settings: Settings, val online: Boolean, val frame: FrameDetections, val nowMs: Long)

    private val local = MutableStateFlow(Local())
    private val effectChannel = Channel<CameraEffect>(Channel.BUFFERED)
    private val photoCache = LinkedHashMap<RecordId, Photo>()
    private val isOnline: StateFlow<Boolean> = online.stateIn(viewModelScope, SharingStarted.Eagerly, true)

    val effects: Flow<CameraEffect> = effectChannel.receiveAsFlow()

    @OptIn(ExperimentalCoroutinesApi::class)
    private val recent: Flow<Recent> =
        combine(captures.observeAll(), local.map { it.toastId }.distinctUntilChanged()) { list, toastId ->
            Recent(list.firstOrNull(), toastId?.let { id -> list.firstOrNull { it.recordId == id } })
        }
            .distinctUntilChanged()
            .mapLatest { r -> r.copy(lastPhoto = r.last?.let { photo(it.recordId) }, toastPhoto = r.toast?.let { photo(it.recordId) }) }

    private val env: Flow<Env> = combine(settings.settings, isOnline, detections, ticks, ::Env)

    val state: StateFlow<CameraUiState> =
        combine(local, recent, env, ::reduce).stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), CameraUiState())

    init {
        viewModelScope.launch { refreshSeal() }
        viewModelScope.launch { refreshLocation() }
    }

    override fun setMode(mode: CaptureMode) {
        viewModelScope.launch { settings.update { it.copy(captureMode = mode) } }
    }

    override fun cycleFlash() = local.update { it.copy(flash = it.flash.next()) }

    override fun flipLens() = local.update { it.copy(lens = it.lens.flipped()) }

    override fun dismissToast() = local.update { it.copy(toastId = null) }

    override fun dismissError() = local.update { it.copy(error = null) }

    override fun shutter() {
        if (local.value.sealing) return
        local.update { it.copy(sealing = true, error = null) }
        viewModelScope.launch {
            try {
                when (val outcome = seal()) {
                    SealOutcome.NoKey -> local.update { it.copy(error = CameraCopy.NO_KEY) }
                    is SealOutcome.Sealed -> onSealed(outcome.capture)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                telemetry.nonFatal(e)
                local.update { it.copy(error = CameraCopy.SEAL_FAILED) }
            } finally {
                local.update { it.copy(sealing = false) }
            }
        }
    }

    private suspend fun onSealed(capture: Capture) {
        val mode = settings.settings.first().captureMode
        if (mode == CaptureMode.POST && isOnline.value) {
            local.update { it.copy(toastId = null) }
            effectChannel.send(CameraEffect.OpenComposer(capture.recordId))
        } else {
            local.update { it.copy(toastId = capture.recordId) }
        }
        viewModelScope.launch { refreshLocation() }
    }

    private suspend fun refreshSeal() {
        val level = try {
            vault.activeKey()?.takeUnless { it.retired }?.security
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        }
        local.update { it.copy(security = level) }
    }

    private suspend fun refreshLocation() {
        val reading = try {
            location.current()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        }
        local.update { it.copy(reading = reading) }
    }

    private suspend fun photo(id: RecordId): Photo? {
        photoCache[id]?.let { return it }
        val loaded = try {
            captures.readImage(id)?.let { Photo(it.toByteArray()) }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        } ?: return null
        if (photoCache.size >= PHOTO_CACHE) photoCache.remove(photoCache.keys.first())
        photoCache[id] = loaded
        return loaded
    }

    private fun reduce(l: Local, r: Recent, e: Env): CameraUiState {
        val mode = e.settings.captureMode
        val detections = CameraCopy.detections(e.frame, mode)
        val toast = r.toast?.let { c ->
            val status = CameraCopy.toastStatus(c.status, e.online)
            SealedToast(c.recordId, CameraCopy.toastTitle(l.security), status.text, status.tone, status.icon, r.toastPhoto)
        }
        return CameraUiState(
            mode = mode,
            seal = CameraCopy.sealPill(l.security),
            location = CameraCopy.locationPill(l.reading),
            online = e.online,
            frameAspect = if (e.frame.dims.isValid && e.frame.proposals.isNotEmpty()) e.frame.dims.width.toFloat() / e.frame.dims.height else null,
            detections = detections,
            hint = if (toast == null) CameraCopy.hint(detections.size, mode, e.settings.defaultPrecision) else null,
            last = r.last?.let { LastCapture(it.recordId, CameraCopy.badge(it.status), r.lastPhoto) },
            toast = toast,
            sealing = l.sealing,
            flash = l.flash,
            lens = l.lens,
            meta = CameraCopy.meta(ClockText.format(e.nowMs, utcOffset(e.nowMs)), e.online, l.reading),
            error = l.error,
        )
    }

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
        const val PHOTO_CACHE: Int = 4
    }
}
