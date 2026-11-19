package app.witness.web

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.navigation3.runtime.NavKey
import app.witness.core.designsystem.WitnessTheme
import app.witness.feature.check.CheckNavigation
import app.witness.feature.check.WebCheckRoute
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.CaseLinkRoute
import app.witness.feature.proof.ProofPageKey
import app.witness.feature.proof.ProofPageRoute
import app.witness.feature.proof.ReportKey
import app.witness.feature.proof.ReportRoute
import app.witness.feature.proof.WebProofNavigation

fun routeFor(key: NavKey): WebRoute? = when (key) {
    is ProofPageKey -> WebRoute.Proof(key.recordId)
    is CaseLinkKey -> WebRoute.Case(key.caseId)
    is ReportKey -> WebRoute.Report(if ('/' in key.page) key.page else WebRoute.PROOF.drop(1) + key.page)
    else -> null
}

@Composable
fun WebApp(router: WebRouter, graph: WebGraph) {
    val proofNav = remember(router) {
        WebProofNavigation(openCheck = { router.navigate(WebRoute.Check) }, navigate = { key -> routeFor(key)?.let(router::navigate) })
    }
    val checkNav = remember(router) {
        CheckNavigation(
            openProof = { router.navigate(WebRoute.Proof(it.text)) },
            openCase = { id, fragment -> router.openCase(id.text, fragment) },
        )
    }
    WitnessTheme {
        when (val route = router.route) {
            is WebRoute.Proof -> ProofPageRoute(route.recordId, graph.proof, proofNav)
            is WebRoute.Case -> CaseLinkRoute(route.caseId, router.fragment, graph.proof, proofNav)
            is WebRoute.Report -> ReportRoute(route.page, graph.proof, proofNav)
            WebRoute.Check, WebRoute.Unknown -> WebCheckRoute(graph.check, checkNav)
        }
    }
}
