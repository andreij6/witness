package app.witness.web

import app.witness.core.domain.Clock
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.icp.witness.BridgeCases
import app.witness.core.icp.witness.BridgeIndex
import app.witness.core.icp.witness.BridgeMedia
import app.witness.core.icp.witness.BridgeRegistry
import app.witness.core.icp.witness.CaseOpening
import app.witness.core.icp.witness.WitnessBridge
import app.witness.core.model.CaseId
import app.witness.core.model.Platform
import app.witness.core.verify.witness.WitnessWebEngine
import app.witness.feature.check.CheckDependencies
import app.witness.feature.proof.CaseLinkOpener
import app.witness.feature.proof.CaseLinkResult
import app.witness.feature.proof.InMemoryReportSender
import app.witness.feature.proof.OpenedCase
import app.witness.feature.proof.OpenedItem
import app.witness.feature.proof.WebProofDependencies

class WebCaseLinks(private val cases: BridgeCases, private val clock: Clock) : CaseLinkOpener {
    override suspend fun open(caseId: CaseId, fragment: String): CaseLinkResult = when (val opened = cases.open(caseId, fragment)) {
        CaseOpening.NotFound -> CaseLinkResult.NotFound
        CaseOpening.Tampered -> CaseLinkResult.Tampered
        is CaseOpening.Opened -> CaseLinkResult.Opened(
            OpenedCase(
                title = opened.title,
                summary = opened.summary,
                sharedAtMs = opened.items.mapNotNull { it.capturedAtMs }.maxOrNull() ?: clock.nowEpochMillis(),
                expiresAtMs = opened.expiresAtMs,
                items = opened.items.map { OpenedItem("", it.jpeg, it.recordId) },
            ),
        )
    }
}

class WebGraph(clock: Clock, bridge: WitnessBridge) {
    val registry = BridgeRegistry(bridge)
    val media = BridgeMedia(bridge)
    val checkImage = CheckImage(WitnessWebEngine(bridge), registry, BridgeIndex, media, Platform.WEB)
    val check: CheckDependencies = CheckDependencies(checkImage, media, clock)
    val proof: WebProofDependencies = WebProofDependencies(registry, media, checkImage, WebCaseLinks(BridgeCases(bridge), clock), InMemoryReportSender())
}
