package app.witness.web

import kotlin.test.Test
import kotlin.test.assertEquals

class WebRouteTest {
    @Test
    fun parsesTheThreeRoutes() {
        assertEquals(WebRoute.Check, WebRoute.parse("/check"))
        assertEquals(WebRoute.Proof("abc123"), WebRoute.parse("/p/abc123"))
        assertEquals(WebRoute.Case("c-9"), WebRoute.parse("/v/c-9"))
    }

    @Test
    fun toleratesTrailingSlash() {
        assertEquals(WebRoute.Check, WebRoute.parse("/check/"))
        assertEquals(WebRoute.Proof("abc"), WebRoute.parse("/p/abc/"))
        assertEquals(WebRoute.Case("abc"), WebRoute.parse("/v/abc//"))
    }

    @Test
    fun rejectsEmptyNestedAndUnknownPaths() {
        listOf("", "/", "/p/", "/p", "/v/", "/p/a/b", "/checks", "/x/abc", "/P/abc", "/check/x", "p/abc").forEach {
            assertEquals(WebRoute.Unknown, WebRoute.parse(it), it)
        }
    }

    @Test
    fun rejectsHostileSegments() {
        listOf("/p/<script>", "/p/a%2Fb", "/p/a.b", "/p/" + "a".repeat(WebRoute.MAX_SEGMENT + 1), "/v/a b", "/p/ä").forEach {
            assertEquals(WebRoute.Unknown, WebRoute.parse(it), it)
        }
        assertEquals(WebRoute.Proof("a".repeat(WebRoute.MAX_SEGMENT)), WebRoute.parse("/p/" + "a".repeat(WebRoute.MAX_SEGMENT)))
    }

    @Test
    fun parsesReportPagesForProofsAndCases() {
        assertEquals(WebRoute.Report("p/abc"), WebRoute.parse("/p/abc/report"))
        assertEquals(WebRoute.Report("v/xyz"), WebRoute.parse("/v/xyz/report/"))
    }

    @Test
    fun rejectsReportsOfUnknownPages() {
        listOf("/report", "/check/report", "/x/report", "/p//report", "/p/a/b/report").forEach {
            assertEquals(WebRoute.Unknown, WebRoute.parse(it), it)
        }
    }

    @Test
    fun pathRoundTrips() {
        listOf(WebRoute.Check, WebRoute.Proof("ab"), WebRoute.Case("cd"), WebRoute.Report("p/ab"), WebRoute.Report("v/cd")).forEach {
            assertEquals(it, WebRoute.parse(it.path), it.toString())
        }
    }

    @Test
    fun unknownFallsBackToTheCheckPath() {
        assertEquals("/check", WebRoute.Unknown.path)
    }
}
