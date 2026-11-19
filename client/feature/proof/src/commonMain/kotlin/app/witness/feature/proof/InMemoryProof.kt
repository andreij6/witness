package app.witness.feature.proof

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

class InMemoryReportSender : ReportSender {
    val sent = ArrayList<PageReport>()

    override suspend fun send(report: PageReport): Boolean {
        sent += report
        return true
    }
}

@OptIn(ExperimentalTime::class)
fun inMemoryWebProofDependencies(clock: Clock = Clock { kotlin.time.Clock.System.now().toEpochMilliseconds() }): WebProofDependencies {
    val registry = FakeRegistry(clock)
    val media = FakeMedia()
    return WebProofDependencies(
        registry,
        media,
        CheckImage(FakeVerifyEngine(), registry, FakeIndex(), media, Platform.WEB),
        { _, _ -> CaseLinkResult.NotFound },
        InMemoryReportSender(),
    )
}

@Deprecated("Register webProofEntries in the app shell", ReplaceWith("webProofEntries(deps, navigation)"))
@Composable
fun ProofScreen(modifier: Modifier = Modifier, recordId: String = "") {
    val deps = remember { inMemoryWebProofDependencies() }
    ProofPageRoute(recordId, deps, remember { WebProofNavigation(openCheck = {}, navigate = {}) }, modifier)
}
