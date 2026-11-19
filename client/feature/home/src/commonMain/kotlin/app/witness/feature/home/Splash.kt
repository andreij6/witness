package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.BiasAlignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessMark
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.domain.port.KeyVault
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class SplashViewModel(private val vault: KeyVault) : ViewModel() {
    private val _destination = MutableStateFlow<HomeKey?>(null)
    val destination: StateFlow<HomeKey?> = _destination.asStateFlow()

    init {
        viewModelScope.launch { _destination.value = route(vault) }
    }

    companion object {
        suspend fun route(vault: KeyVault): HomeKey = if (vault.activeKey()?.takeUnless { it.retired } != null) HomeKey.Home else HomeKey.Welcome
    }
}

@Composable
fun SplashRoute(viewModel: SplashViewModel, navigator: HomeNavigator) {
    val destination by viewModel.destination.collectAsStateWithLifecycle()
    LaunchedEffect(destination) { destination?.let(navigator.resetTo) }
    SplashScreen()
}

const val SPLASH_TAGLINE = "Proof, not just pictures"
const val SPLASH_FOOTER = "Sealed on your phone · anchored on the Internet Computer"

@Composable
fun SplashScreen(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxSize().background(WitnessColors.Ink).windowInsetsPadding(WindowInsets.safeDrawing)) {
        Column(Modifier.align(BiasAlignment(0f, -0.07f)), horizontalAlignment = Alignment.CenterHorizontally) {
            WitnessMark(size = 120.dp, ring = WitnessColors.White, check = WitnessColors.IconCheck, contentDescription = null)
            VSpace(26.dp)
            Text("Witness", style = Witness.type.hero.copy(fontSize = 36.sp), color = WitnessColors.White)
            VSpace(4.dp)
            Text(SPLASH_TAGLINE, style = Witness.type.bodyLarge, color = WitnessColors.CameraInk2)
        }
        Text(
            SPLASH_FOOTER,
            Modifier.align(Alignment.BottomCenter).padding(start = Gutter, end = Gutter, bottom = 40.dp),
            style = Witness.type.proofSmall,
            color = WitnessColors.CameraMuted,
            textAlign = TextAlign.Center,
        )
    }
}

@Preview
@Composable
private fun SplashPreview() = WitnessTheme { SplashScreen() }
