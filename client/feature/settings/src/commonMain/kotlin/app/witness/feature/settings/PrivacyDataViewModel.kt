package app.witness.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.Capture
import app.witness.core.model.RecordId
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

enum class PrivacyBusy { Exporting, Erasing }

data class PrivacyDataUiState(
    val busy: PrivacyBusy? = null,
    val confirmErase: Boolean = false,
    val notice: Notice? = null,
    val items: List<PublicItemUi> = emptyList(),
    val confirmItem: PublicItemUi? = null,
    val erasingItem: RecordId? = null,
)

class PrivacyDataViewModel(
    private val exporter: DataExporter,
    captures: Flow<List<Capture>> = emptyFlow(),
    private val eraseItem: suspend (RecordId) -> EraseReport = { EraseReport() },
    private val eraseAll: suspend () -> EraseReport,
) : ViewModel() {
    private val mutableState = MutableStateFlow(PrivacyDataUiState())

    val state: StateFlow<PrivacyDataUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch { captures.collect { all -> mutableState.update { it.copy(items = PublicItems.of(all)) } } }
    }

    fun export() {
        if (!claim(PrivacyBusy.Exporting)) return
        viewModelScope.launch {
            val outcome = attempt { exporter.export() }.getOrElse { ExportOutcome.Failed(it.message.orEmpty()) }
            mutableState.update { it.copy(busy = null, notice = SettingsCopy.exportNotice(outcome)) }
        }
    }

    fun requestErase() = mutableState.update { if (it.busy == null) it.copy(confirmErase = true, confirmItem = null) else it }

    fun cancelErase() = mutableState.update { it.copy(confirmErase = false, confirmItem = null) }

    fun confirmErase() {
        if (!mutableState.value.confirmErase || !claim(PrivacyBusy.Erasing)) return
        viewModelScope.launch {
            val report = attempt { eraseAll() }.getOrElse { EraseReport(offline = true) }
            mutableState.update { it.copy(busy = null, notice = SettingsCopy.eraseNotice(report)) }
        }
    }

    fun requestEraseItem(recordId: RecordId) = mutableState.update { s ->
        val item = s.items.firstOrNull { it.recordId == recordId }
        if (s.busy == null && item != null) s.copy(confirmItem = item, confirmErase = false) else s
    }

    fun confirmEraseItem() {
        val item = mutableState.value.confirmItem ?: return
        if (!claim(PrivacyBusy.Erasing, item.recordId)) return
        viewModelScope.launch {
            val report = attempt { eraseItem(item.recordId) }.getOrElse { EraseReport(offline = true) }
            mutableState.update { it.copy(busy = null, erasingItem = null, notice = SettingsCopy.eraseNotice(report)) }
        }
    }

    fun dismissNotice() = mutableState.update { it.copy(notice = null) }

    private fun claim(busy: PrivacyBusy, item: RecordId? = null): Boolean {
        var claimed = false
        mutableState.update {
            claimed = it.busy == null
            if (claimed) it.copy(busy = busy, confirmErase = false, confirmItem = null, erasingItem = item, notice = null) else it
        }
        return claimed
    }
}
