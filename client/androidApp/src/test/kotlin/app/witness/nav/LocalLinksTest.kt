package app.witness.nav

import app.witness.BuildConfig
import app.witness.core.domain.port.Links
import app.witness.core.model.CaseId
import app.witness.core.model.RecordId
import app.witness.core.platform.share.SharedItem
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ProofPageKey
import java.io.File
import kotlin.io.encoding.Base64
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class LocalLinksTest {
    private val record = RecordId.fromDigest(ByteArray(32) { (it * 7).toByte() })
    private val caseId = CaseId.of(1, ByteArray(CaseId.RANDOM_SIZE) { (it * 3).toByte() })
    private val key = Base64.UrlSafe.withPadding(Base64.PaddingOption.ABSENT).encode(ByteArray(32) { it.toByte() })
    private val local = Links.at("http://4qggx-l3777-77775-aaaca-cai.localhost:8090")

    @Test
    fun localBuildsLinkToTheLocalPagesCanisterAndReleaseToProduction() {
        val links = Links.at(BuildConfig.PAGES_ORIGIN)
        if (BuildConfig.ICP_FETCH_ROOT_KEY && BuildConfig.PAGES_ORIGIN.isNotEmpty()) {
            val pages = pagesUrl()
            if (pages != null) assertEquals(pages.trimEnd('/') + "/p/", links.proofBase)
            assertTrue(links.proofBase.startsWith("http://") && ".localhost:" in links.proofBase, links.proofBase)
            assertTrue(links.caseBase.endsWith("/v/"), links.caseBase)
        } else {
            assertEquals(Links(), links)
        }
    }

    @Test
    fun localLinksOpenTheirPagesInApp() {
        assertEquals(ProofPageKey(record.text), DeepLinks.keyFor(local.proof(record), local))
        val k = assertIs<CaseLinkKey>(DeepLinks.keyFor(local.caseBase + caseId.text + "#k=" + key, local))
        assertEquals(caseId.text, k.caseId)
        assertEquals(ProofPageKey(record.text), DeepLinks.keyFor(SharedItem.ProofLink(local.proof(record), record), { null }, local))
    }

    @Test
    fun localLinksAreForeignToProductionAndViceVersa() {
        assertNull(DeepLinks.keyFor(local.proof(record)))
        assertNull(DeepLinks.keyFor(Links().proof(record), local))
        assertNull(DeepLinks.keyFor("http://other-canister.localhost:8090/p/${record.text}", local))
    }

    @Test
    fun navigatorSharesTheConfiguredLink() {
        val shared = mutableListOf<String>()
        val nav = AppNavigator(mutableListOf(), RecordingOutside(shared), local)
        nav.item.share(record)
        assertEquals(listOf(local.proof(record)), shared)
    }

    private fun pagesUrl(): String? {
        val ids = File("../../.icp-local/ids.json").takeIf(File::isFile)?.readText() ?: return null
        return Regex(""""pages"\s*:\s*\{[^}]*"url"\s*:\s*"([^"]+)"""").find(ids)?.groupValues?.get(1)
    }

    private class RecordingOutside(private val texts: MutableList<String>) : Outside {
        override fun shareText(text: String) {
            texts += text
        }

        override fun openUrl(url: String) = Unit

        override fun copy(text: String) = Unit

        override fun shareFile(name: String, mime: String, bytes: ByteArray) = Unit

        override fun sharePost(text: String, record: RecordId) = Unit

        override fun captureInto(case: app.witness.core.model.LocalCaseId) = Unit

        override fun finish() = Unit
    }
}
