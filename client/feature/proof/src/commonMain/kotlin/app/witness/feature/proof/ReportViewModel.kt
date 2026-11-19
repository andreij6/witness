package app.witness.feature.proof

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

enum class ReportReason(val label: String) {
    PRIVATE_INFO("Someone's private information"),
    SEXUAL("Nudity or sexual content"),
    VIOLENCE("Violence or abuse"),
    COPYRIGHT("Copyright"),
    MISLEADING("Misleading or fake"),
    OTHER("Other"),
}

data class PageReport(val page: String, val reason: ReportReason, val details: String, val email: String?)

fun interface ReportSender {
    suspend fun send(report: PageReport): Boolean
}

enum class ReportPhase { Editing, Sending, Sent, Failed }

@Immutable
data class ReportUiState(
    val reason: ReportReason = ReportReason.PRIVATE_INFO,
    val details: String = "",
    val email: String = "",
    val phase: ReportPhase = ReportPhase.Editing,
    val emailError: String? = null,
) {
    val canSend: Boolean get() = phase == ReportPhase.Editing || phase == ReportPhase.Failed
}

class ReportViewModel(private val page: String, private val sender: ReportSender) : ViewModel() {
    private val mutable = MutableStateFlow(ReportUiState())
    val state: StateFlow<ReportUiState> = mutable.asStateFlow()

    fun select(reason: ReportReason) = edit { it.copy(reason = reason) }

    fun onDetails(text: String) = edit { it.copy(details = text.take(MAX_DETAILS)) }

    fun onEmail(text: String) = edit { it.copy(email = text.take(MAX_EMAIL).trim(), emailError = null) }

    private inline fun edit(crossinline f: (ReportUiState) -> ReportUiState) = mutable.update { if (it.canSend) f(it) else it }

    fun send() {
        val s = state.value
        if (!s.canSend) return
        val email = s.email.takeIf { it.isNotEmpty() }
        if (email != null && !isEmail(email)) {
            mutable.update { it.copy(emailError = "Enter a valid email, or leave it empty") }
            return
        }
        mutable.update { it.copy(phase = ReportPhase.Sending) }
        viewModelScope.launch {
            val ok = try {
                sender.send(PageReport(page, s.reason, s.details.trim(), email))
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                false
            }
            mutable.update { it.copy(phase = if (ok) ReportPhase.Sent else ReportPhase.Failed) }
        }
    }

    companion object {
        const val MAX_DETAILS: Int = 1000
        const val MAX_EMAIL: Int = 254

        fun isEmail(text: String): Boolean {
            if (text.length > MAX_EMAIL || text.any { it.isWhitespace() }) return false
            val at = text.indexOf('@')
            if (at <= 0 || at != text.lastIndexOf('@')) return false
            val domain = text.substring(at + 1)
            val dot = domain.lastIndexOf('.')
            return dot > 0 && dot < domain.length - 2 && !domain.startsWith('.') && ".." !in domain
        }
    }
}
