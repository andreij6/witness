package app.witness.nav

import androidx.navigation3.runtime.NavKey
import app.witness.core.domain.port.Links
import app.witness.core.model.CaseId
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.feature.capture.CameraKey
import app.witness.feature.cases.CaseKey
import app.witness.feature.cases.CaseNavigator
import app.witness.feature.cases.CaseSwitcherKey
import app.witness.feature.cases.NewCaseKey
import app.witness.feature.check.CheckKey
import app.witness.feature.check.CheckNavigation
import app.witness.feature.home.HomeKey
import app.witness.feature.home.HomeNavigator
import app.witness.feature.post.PostComposerKey
import app.witness.feature.post.PostSharedKey
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ItemNavigation
import app.witness.feature.proof.ItemProofKey
import app.witness.feature.proof.ProofPageKey
import app.witness.feature.proof.WebProofNavigation
import app.witness.feature.settings.SettingsKey
import app.witness.feature.settings.SettingsNavigator
import app.witness.feature.share.ExportCaseKey
import app.witness.feature.share.ExportResult
import app.witness.feature.share.ShareCaseKey
import app.witness.feature.share.ShareNavigator

interface Outside {
    fun shareText(text: String)

    fun openUrl(url: String)

    fun copy(text: String)

    fun shareFile(name: String, mime: String, bytes: ByteArray)

    fun sharePost(text: String, record: RecordId)

    fun captureInto(case: LocalCaseId)

    fun finish()
}

class AppNavigator(
    private val stack: MutableList<NavKey>,
    private val outside: Outside,
    private val links: Links = Links(),
) : CaseNavigator, ShareNavigator, SettingsNavigator {
    private var deferred: NavKey? = null

    val top: NavKey? get() = stack.lastOrNull()

    fun push(key: NavKey) {
        if (stack.lastOrNull() != key) stack.add(key)
    }

    fun resetTo(key: NavKey) {
        val pending = deferred
        deferred = null
        stack.clear()
        stack.add(key)
        if (pending != null && pending != key) stack.add(pending)
    }

    override fun back() {
        if (stack.size > 1) stack.removeAt(stack.lastIndex) else outside.finish()
    }

    fun open(link: NavKey) {
        if (stack.isEmpty() || stack.last() == HomeKey.Splash) deferred = link else push(link)
    }

    fun selectTab(key: NavKey) {
        val at = stack.lastIndexOf(key)
        if (at < 0) {
            push(key)
            return
        }
        while (stack.lastIndex > at) stack.removeAt(stack.lastIndex)
    }

    fun popTo(predicate: (NavKey) -> Boolean): Boolean {
        val at = stack.indexOfLast(predicate)
        if (at < 0) return false
        while (stack.lastIndex > at) stack.removeAt(stack.lastIndex)
        return true
    }

    val home: HomeNavigator = HomeNavigator(
        push = ::push,
        resetTo = ::resetTo,
        back = ::back,
        openCamera = { push(CameraKey) },
        openCheck = { selectTab(CheckKey) },
        openSettings = { push(SettingsKey.Root) },
        openItem = ::openItem,
        openCase = ::openCase,
        newCase = ::newCase,
    )

    val item: ItemNavigation = ItemNavigation(
        back = ::back,
        navigate = ::push,
        share = { outside.shareText(links.proof(it)) },
        createPost = { push(PostComposerKey.of(it)) },
        shareFile = outside::shareFile,
    )

    val check: CheckNavigation = CheckNavigation(
        openProof = { push(ProofPageKey(it.text)) },
        openCase = ::openCaseLink,
        close = ::back,
        share = outside::shareText,
    )

    val proof: WebProofNavigation = WebProofNavigation(openCheck = { selectTab(CheckKey) }, navigate = ::push)

    fun openCaseLink(caseId: CaseId, fragment: String) = push(CaseLinkKey(caseId.text, fragment))

    fun composed(recordId: RecordId) {
        popTo { it == CameraKey }
        push(PostComposerKey.of(recordId))
    }

    fun shared(key: PostSharedKey) {
        if (stack.lastOrNull() is PostComposerKey) stack.removeAt(stack.lastIndex)
        push(key)
    }

    fun postDone() {
        if (!popTo { it == HomeKey.Home }) resetTo(HomeKey.Home)
    }

    override fun openCase(id: LocalCaseId) = push(CaseKey(id.value))

    override fun newCase() = push(NewCaseKey)

    override fun switchCase() = push(CaseSwitcherKey)

    override fun caseCreated(id: LocalCaseId) {
        if (stack.lastOrNull() == NewCaseKey) stack.removeAt(stack.lastIndex)
        if (stack.lastOrNull() == CaseSwitcherKey) stack.removeAt(stack.lastIndex)
        if (stack.lastOrNull() != CameraKey) push(CaseKey(id.value))
    }

    override fun shareCase(id: LocalCaseId) = push(ShareCaseKey(id.value))

    override fun exportCase(id: LocalCaseId) = push(ExportCaseKey(id.value))

    override fun captureFor(id: LocalCaseId, target: String?) {
        outside.captureInto(id)
        push(CameraKey)
    }

    override fun openItem(recordId: RecordId) = push(ItemProofKey(recordId.text))

    override fun search() = push(HomeKey.Search)

    override fun sendLink(url: String) = outside.shareText(url)

    override fun copyLink(url: String) = outside.copy(url)

    override fun preview(url: String) = outside.openUrl(url)

    override fun exported(result: ExportResult.Saved) = back()

    override fun upgrade() {
        back()
        push(SettingsKey.Plus)
    }

    override fun navigate(key: SettingsKey) = push(key)

    override fun openExternal(url: String) = outside.openUrl(url)
}
