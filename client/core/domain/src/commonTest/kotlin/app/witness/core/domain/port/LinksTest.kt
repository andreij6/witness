package app.witness.core.domain.port

import app.witness.core.model.RecordId
import kotlin.test.Test
import kotlin.test.assertEquals

class LinksTest {
    private val id = RecordId.parse("WERC8GMRZGE196QV")

    @Test
    fun blankOriginKeepsProductionLinks() {
        assertEquals(Links(), Links.at(""))
        assertEquals(Links(), Links.at("   "))
        assertEquals(Links(), Links.at("/"))
        assertEquals("https://witness.app/p/${id.text}", Links.at("").proof(id))
    }

    @Test
    fun localOriginPointsProofAndCaseLinksAtIt() {
        val local = Links.at("http://4qggx-l3777-77775-aaaca-cai.localhost:8090")
        assertEquals("http://4qggx-l3777-77775-aaaca-cai.localhost:8090/p/", local.proofBase)
        assertEquals("http://4qggx-l3777-77775-aaaca-cai.localhost:8090/v/", local.caseBase)
        assertEquals("http://4qggx-l3777-77775-aaaca-cai.localhost:8090/p/${id.text}", local.proof(id))
    }

    @Test
    fun trailingSlashesAndWhitespaceAreNormalised() {
        assertEquals(Links.at("http://a.localhost:8090"), Links.at(" http://a.localhost:8090// "))
    }

    @Test
    fun isDeterministic() {
        assertEquals(Links.at("http://a.localhost:8090"), Links.at("http://a.localhost:8090"))
    }
}
