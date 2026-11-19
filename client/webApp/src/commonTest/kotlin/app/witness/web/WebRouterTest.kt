package app.witness.web

import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ItemProofKey
import app.witness.feature.proof.ProofPageKey
import app.witness.feature.proof.ReportKey
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class FakeHistory(override var pathname: String, override var hash: String = "") : History {
    val pushed = ArrayList<String>()
    val replaced = ArrayList<String>()

    override fun push(path: String) {
        pushed += path
        pathname = path
        hash = ""
    }

    override fun replace(path: String) {
        replaced += path
        pathname = path
        hash = ""
    }

    fun back(path: String, fragment: String = "") {
        pathname = path
        hash = fragment
    }
}

class MapKeys : KeyStore {
    val map = HashMap<String, String>()
    override fun get(name: String): String? = map[name]
    override fun put(name: String, value: String) {
        map[name] = value
    }
}

class WebRouterTest {
    private val keys = MapKeys()

    @Test
    fun startsOnTheParsedRoute() {
        assertEquals(WebRoute.Proof("ab"), WebRouter(FakeHistory("/p/ab"), keys).route)
        assertEquals(WebRoute.Check, WebRouter(FakeHistory("/check"), keys).route)
        assertEquals(WebRoute.Unknown, WebRouter(FakeHistory("/"), keys).route)
    }

    @Test
    fun shellSummaryWinsForProofAndUnknownPaths() {
        val shell = ShellSummary("REC1", 2)
        assertEquals(WebRoute.Proof("REC1"), WebRouter(FakeHistory("/p/other"), keys, shell).route)
        assertEquals(WebRoute.Proof("REC1"), WebRouter(FakeHistory("/"), keys, shell).route)
        assertEquals(WebRoute.Check, WebRouter(FakeHistory("/check"), keys, shell).route)
        assertEquals(WebRoute.Proof("x"), WebRouter(FakeHistory("/p/x"), keys, ShellSummary(null, 1)).route)
    }

    @Test
    fun caseKeyMovesFromTheUrlToSessionStorage() {
        val history = FakeHistory("/v/CASE", "#k=secret")
        val router = WebRouter(history, keys)
        assertEquals("k=secret", router.fragment)
        assertEquals("k=secret", keys.map[WebRouter.keyName("CASE")])
        assertEquals(listOf("/v/CASE"), history.replaced)
        assertEquals("", history.hash)
    }

    @Test
    fun reloadWithoutHashRecoversTheKey() {
        keys.put(WebRouter.keyName("CASE"), "k=saved")
        val history = FakeHistory("/v/CASE")
        val router = WebRouter(history, keys)
        assertEquals("k=saved", router.fragment)
        assertTrue(history.replaced.isEmpty())
    }

    @Test
    fun caseWithoutAnyKeyHasEmptyFragment() {
        assertEquals("", WebRouter(FakeHistory("/v/CASE"), keys).fragment)
    }

    @Test
    fun hashOnNonCasePagesIsIgnored() {
        val history = FakeHistory("/p/ab", "#k=x")
        val router = WebRouter(history, keys)
        assertEquals("", router.fragment)
        assertTrue(keys.map.isEmpty())
        assertTrue(history.replaced.isEmpty())
    }

    @Test
    fun navigatePushesHistoryOnce() {
        val history = FakeHistory("/check")
        val router = WebRouter(history, keys)
        router.navigate(WebRoute.Proof("ab"))
        router.navigate(WebRoute.Proof("ab"))
        assertEquals(WebRoute.Proof("ab"), router.route)
        assertEquals(listOf("/p/ab"), history.pushed)
    }

    @Test
    fun openCaseStoresTheKeyAndKeepsItOutOfTheUrl() {
        val history = FakeHistory("/check")
        val router = WebRouter(history, keys)
        router.openCase("CASE", "#k=abc")
        assertEquals(WebRoute.Case("CASE"), router.route)
        assertEquals("k=abc", router.fragment)
        assertEquals(listOf("/v/CASE"), history.pushed)
        assertTrue(history.pushed.none { "k=" in it })
    }

    @Test
    fun backButtonRestoresTheRouteAndKey() {
        val history = FakeHistory("/v/CASE", "#k=one")
        val router = WebRouter(history, keys)
        router.navigate(WebRoute.Proof("ab"))
        assertEquals("", router.fragment)
        history.back("/v/CASE")
        router.onPop()
        assertEquals(WebRoute.Case("CASE"), router.route)
        assertEquals("k=one", router.fragment)
    }

    @Test
    fun navigatingBackToACaseReusesItsStoredKey() {
        val router = WebRouter(FakeHistory("/v/CASE", "#k=one"), keys)
        router.navigate(WebRoute.Check)
        router.navigate(WebRoute.Case("CASE"))
        assertEquals("k=one", router.fragment)
    }

    @Test
    fun navKeysMapToRoutes() {
        assertEquals(WebRoute.Proof("ab"), routeFor(ProofPageKey("ab")))
        assertEquals(WebRoute.Case("cd"), routeFor(CaseLinkKey("cd", "k=x")))
        assertEquals(WebRoute.Report("v/cd"), routeFor(ReportKey("v/cd")))
        assertEquals(WebRoute.Report("p/ab"), routeFor(ReportKey("ab")))
        assertNull(routeFor(ItemProofKey("ab")))
    }
}
