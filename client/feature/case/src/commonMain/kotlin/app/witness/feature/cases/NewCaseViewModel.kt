package app.witness.feature.cases

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.domain.usecase.TemplateMatcher
import app.witness.core.model.LocalCaseId
import app.witness.core.model.TemplateSchema
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class NewCaseUiState(
    val name: String = "",
    val selected: ReportType? = null,
    val manual: Boolean = false,
    val creating: Boolean = false,
    val created: LocalCaseId? = null,
    val error: String? = null,
) {
    val canCreate: Boolean get() = name.isNotBlank() && !creating && created == null
}

class NewCaseViewModel(
    private val manageCases: ManageCases,
    catalog: TemplateCatalog,
    private val activate: Boolean = true,
) : ViewModel() {
    private val mutable = MutableStateFlow(NewCaseUiState())
    val state: StateFlow<NewCaseUiState> = mutable.asStateFlow()

    private var templates: List<TemplateSchema> = emptyList()

    init {
        viewModelScope.launch {
            templates = try {
                catalog.templates()
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                emptyList()
            }
            mutable.update { it.copy(selected = if (it.manual) it.selected else suggest(it.name, templates)) }
        }
    }

    fun setName(text: String) = mutable.update {
        val name = text.take(ManageCases.MAX_TITLE)
        it.copy(name = name, selected = if (it.manual) it.selected else suggest(name, templates), error = null)
    }

    fun select(type: ReportType) {
        if (!type.available) return
        mutable.update { it.copy(selected = type, manual = true) }
    }

    fun create() {
        val current = mutable.value
        if (!current.canCreate) return
        mutable.update { it.copy(creating = true, error = null) }
        viewModelScope.launch {
            try {
                val case = manageCases.create(current.name, current.selected?.templateId?.takeIf { current.manual }, activate)
                mutable.update { it.copy(creating = false, created = case.id) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                mutable.update { it.copy(creating = false, error = e.message ?: CaseReportViewModel.GENERIC_ERROR) }
            }
        }
    }

    companion object {
        fun suggest(name: String, templates: List<TemplateSchema>): ReportType? =
            if (name.isBlank()) null else ReportType.of(TemplateMatcher.pick(name, templates)) ?: ReportType.GENERAL
    }
}
