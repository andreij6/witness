package app.witness.feature.check

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import app.witness.core.domain.Clock
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeVerifyEngine
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.model.Platform
import kotlin.time.ExperimentalTime

@OptIn(ExperimentalTime::class)
val SystemClock: Clock = Clock { kotlin.time.Clock.System.now().toEpochMilliseconds() }

fun inMemoryCheckDependencies(platform: Platform, clock: Clock = SystemClock): CheckDependencies {
    val media = FakeMedia()
    return CheckDependencies(CheckImage(FakeVerifyEngine(), FakeRegistry(clock), FakeIndex(), media, platform), media, clock)
}

@Deprecated("Register checkEntries in the app shell", ReplaceWith("checkEntries(deps, navigation)"))
@Composable
fun CheckScreen(modifier: Modifier = Modifier) {
    val deps = remember { inMemoryCheckDependencies(Platform.WEB) }
    WebCheckRoute(deps, remember { CheckNavigation(openProof = {}, openCase = { _, _ -> }) }, modifier)
}
