package app.witness.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class IdentityLinkUiState(val linked: Principal? = null, val status: LinkStatus = LinkStatus.Idle) {
    val waiting: Boolean get() = status == LinkStatus.Waiting || status == LinkStatus.Linking

    val error: String? get() = (status as? LinkStatus.Failed)?.let { SettingsCopy.linkError(it.error) }
}

sealed interface IdentityEffect {
    data class OpenBrowser(val url: String) : IdentityEffect

    data object Close : IdentityEffect
}

class IdentityLinkViewModel(private val flow: IdentityLinkFlow, private val identity: IdentityAccount) : ViewModel() {
    private val effects = Channel<IdentityEffect>(Channel.BUFFERED)

    val effect: Flow<IdentityEffect> = effects.receiveAsFlow()

    val state: StateFlow<IdentityLinkUiState> = combine(identity.principal, flow.status, ::IdentityLinkUiState)
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), IdentityLinkUiState(status = flow.status.value))

    fun start() {
        effects.trySend(IdentityEffect.OpenBrowser(flow.begin()))
    }

    fun notNow() {
        flow.reset()
        effects.trySend(IdentityEffect.Close)
    }

    fun dismissError() {
        if (flow.status.value is LinkStatus.Failed) flow.reset()
    }

    fun unlink() {
        viewModelScope.launch {
            attempt { identity.unlink() }
            flow.reset()
        }
    }
}
