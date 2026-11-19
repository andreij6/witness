package app.witness.nav

import androidx.navigation3.runtime.NavKey
import app.witness.core.model.CaseId
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import app.witness.feature.capture.CameraKey
import app.witness.feature.cases.CaseKey
import app.witness.feature.cases.CaseSwitcherKey
import app.witness.feature.cases.NewCaseKey
import app.witness.feature.check.CheckKey
import app.witness.feature.home.HomeKey
import app.witness.feature.post.PostComposerKey
import app.witness.feature.post.PostSharedKey
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ItemProofKey
import app.witness.feature.proof.ProofPageKey
import app.witness.feature.proof.TechnicalRecordKey
import app.witness.feature.settings.SettingsKey
import app.witness.feature.share.ExportCaseKey
import app.witness.feature.share.ExportResult
import app.witness.feature.share.ShareCaseKey
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class RecordingOutside : Outside {
    val calls = ArrayList<String>()
    override fun shareText(text: String) { calls += "share:$text" }
    override fun openUrl(url: String) { calls += "open:$url" }
    override fun copy(text: String) { calls += "copy:$text" }
    override fun shareFile(name: String, mime: String, bytes: ByteArray) { calls += "file:$name:$mime:${bytes.size}" }
    override fun sharePost(text: String, record: RecordId) { calls += "post:${record.text}:$text" }
    override fun captureInto(case: LocalCaseId) { calls += "capture:${case.value}" }
    override fun finish() { calls += "finish" }
}

class AppNavigatorTest {
    private val record = RecordId.fromDigest(ByteArray(32) { it.toByte() })
    private val outside = RecordingOutside()
    private val stack = mutableListOf<NavKey>(HomeKey.Splash)
    private val nav = AppNavigator(stack, outside)

    private fun at(vararg keys: NavKey) {
        stack.clear()
        stack.addAll(keys)
    }

    @Test
    fun pushIgnoresTheKeyAlreadyOnTop() {
        nav.push(HomeKey.Welcome)
        nav.push(HomeKey.Welcome)
        assertEquals(listOf<NavKey>(HomeKey.Splash, HomeKey.Welcome), stack)
    }

    @Test
    fun resetToReplacesTheWholeStack() {
        at(HomeKey.Splash, HomeKey.Welcome, HomeKey.Permissions)
        nav.resetTo(HomeKey.Home)
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
    }

    @Test
    fun deepLinkDuringSplashWaitsForTheFirstReset() {
        nav.open(CheckKey)
        assertEquals(listOf<NavKey>(HomeKey.Splash), stack)
        nav.resetTo(HomeKey.Home)
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
        nav.resetTo(HomeKey.Home)
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
    }

    @Test
    fun deferredLinkEqualToTheResetTargetIsNotDuplicated() {
        nav.open(HomeKey.Home)
        nav.resetTo(HomeKey.Home)
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
    }

    @Test
    fun laterDeepLinkReplacesAnEarlierDeferredOne() {
        nav.open(CheckKey)
        nav.open(ProofPageKey(record.text))
        nav.resetTo(HomeKey.Welcome)
        assertEquals(listOf(HomeKey.Welcome, ProofPageKey(record.text)), stack)
    }

    @Test
    fun deepLinkAfterSplashPushesImmediately() {
        at(HomeKey.Home)
        nav.open(CheckKey)
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
    }

    @Test
    fun deepLinkOnEmptyStackIsDeferred() {
        stack.clear()
        nav.open(CheckKey)
        nav.resetTo(HomeKey.Home)
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
    }

    @Test
    fun backPopsUntilRootThenFinishes() {
        at(HomeKey.Home, CheckKey)
        nav.back()
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
        assertTrue(outside.calls.isEmpty())
        nav.back()
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
        assertEquals(listOf("finish"), outside.calls)
    }

    @Test
    fun selectTabPopsBackToAnExistingTab() {
        at(HomeKey.Home, CheckKey, ProofPageKey(record.text))
        nav.selectTab(HomeKey.Home)
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
    }

    @Test
    fun selectTabPushesAMissingTab() {
        at(HomeKey.Home)
        nav.selectTab(CheckKey)
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
    }

    @Test
    fun selectTabOnTheCurrentTabIsANoOp() {
        at(HomeKey.Home, CheckKey)
        nav.selectTab(CheckKey)
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
    }

    @Test
    fun popToReportsWhetherTheTargetWasFound() {
        at(HomeKey.Home, CameraKey, CheckKey)
        assertEquals(false, nav.popTo { it == SettingsKey.Root })
        assertEquals(3, stack.size)
        assertEquals(true, nav.popTo { it == CameraKey })
        assertEquals(listOf(HomeKey.Home, CameraKey), stack)
    }

    @Test
    fun homeNavigatorRoutesEveryAction() {
        at(HomeKey.Home)
        nav.home.openCamera()
        assertEquals(CameraKey, nav.top)
        nav.home.openSettings()
        assertEquals(SettingsKey.Root, nav.top)
        nav.home.openItem(record)
        assertEquals(ItemProofKey(record.text), nav.top)
        nav.home.openCase(LocalCaseId(7))
        assertEquals(CaseKey(7), nav.top)
        nav.home.newCase()
        assertEquals(NewCaseKey, nav.top)
        nav.home.openCheck()
        assertEquals(CheckKey, nav.top)
    }

    @Test
    fun finishingOnboardingLandsOnHomeAndCanOpenTheCamera() {
        at(HomeKey.Welcome, HomeKey.Permissions, HomeKey.DeviceSetup)
        nav.home.finishOnboarding(openCamera = true)
        assertEquals(listOf(HomeKey.Home, CameraKey), stack)
    }

    @Test
    fun composerOpensOverTheCameraNotOverStaleScreens() {
        at(HomeKey.Home, CameraKey, ItemProofKey(record.text))
        nav.composed(record)
        assertEquals(listOf(HomeKey.Home, CameraKey, PostComposerKey.of(record)), stack)
    }

    @Test
    fun sharedReplacesTheComposer() {
        at(HomeKey.Home, CameraKey, PostComposerKey.of(record))
        val shared = PostSharedKey.of(record, LocationPrecision.AREA)
        nav.shared(shared)
        assertEquals(listOf(HomeKey.Home, CameraKey, shared), stack)
    }

    @Test
    fun sharedWithoutComposerJustPushes() {
        at(HomeKey.Home)
        val shared = PostSharedKey.of(record, LocationPrecision.HIDDEN)
        nav.shared(shared)
        assertEquals(listOf(HomeKey.Home, shared), stack)
    }

    @Test
    fun postDoneReturnsHomeOrResetsWhenHomeIsAbsent() {
        at(HomeKey.Home, CameraKey, PostSharedKey.of(record, LocationPrecision.AREA))
        nav.postDone()
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
        at(CheckKey, PostSharedKey.of(record, LocationPrecision.AREA))
        nav.postDone()
        assertEquals(listOf<NavKey>(HomeKey.Home), stack)
    }

    @Test
    fun caseCreatedFromTheCameraReturnsToTheCamera() {
        at(HomeKey.Home, CameraKey, CaseSwitcherKey, NewCaseKey)
        nav.caseCreated(LocalCaseId(3))
        assertEquals(listOf(HomeKey.Home, CameraKey), stack)
    }

    @Test
    fun caseCreatedFromHomeOpensTheCase() {
        at(HomeKey.Home, NewCaseKey)
        nav.caseCreated(LocalCaseId(3))
        assertEquals(listOf(HomeKey.Home, CaseKey(3)), stack)
    }

    @Test
    fun captureForActivatesTheCaseThenOpensTheCamera() {
        at(HomeKey.Home, CaseKey(4))
        nav.captureFor(LocalCaseId(4), "plate")
        assertEquals(CameraKey, nav.top)
        assertEquals(listOf("capture:4"), outside.calls)
    }

    @Test
    fun caseNavigatorRoutes() {
        at(HomeKey.Home)
        nav.shareCase(LocalCaseId(2))
        assertEquals(ShareCaseKey(2), nav.top)
        nav.exportCase(LocalCaseId(2))
        assertEquals(ExportCaseKey(2), nav.top)
        nav.switchCase()
        assertEquals(CaseSwitcherKey, nav.top)
        nav.search()
        assertEquals(HomeKey.Search, nav.top)
        nav.openItem(record)
        assertEquals(ItemProofKey(record.text), nav.top)
    }

    @Test
    fun shareNavigatorUsesTheOutsideWorld() {
        at(HomeKey.Home, ShareCaseKey(1))
        nav.sendLink("https://case.witness.app/v/x#k=y")
        nav.copyLink("u")
        nav.preview("p")
        assertEquals(listOf("share:https://case.witness.app/v/x#k=y", "copy:u", "open:p"), outside.calls)
    }

    @Test
    fun exportedClosesTheSheet() {
        at(HomeKey.Home, CaseKey(1), ExportCaseKey(1))
        nav.exported(ExportResult.Saved("a.zip", "Downloads/a.zip", 10, 1))
        assertEquals(listOf(HomeKey.Home, CaseKey(1)), stack)
    }

    @Test
    fun upgradeClosesTheSheetAndOpensPlus() {
        at(HomeKey.Home, CaseKey(1), ExportCaseKey(1))
        nav.upgrade()
        assertEquals(listOf(HomeKey.Home, CaseKey(1), SettingsKey.Plus), stack)
    }

    @Test
    fun settingsNavigatorRoutes() {
        at(HomeKey.Home, SettingsKey.Root)
        nav.navigate(SettingsKey.PrivacyData)
        assertEquals(SettingsKey.PrivacyData, nav.top)
        nav.openExternal("https://witness.app/terms")
        assertEquals(listOf("open:https://witness.app/terms"), outside.calls)
    }

    @Test
    fun itemNavigationSharesTheProofLinkAndOpensPosts() {
        at(HomeKey.Home, ItemProofKey(record.text))
        nav.item.share(record)
        assertEquals(listOf("share:https://witness.app/p/${record.text}"), outside.calls)
        nav.item.createPost(record)
        assertEquals(PostComposerKey.of(record), nav.top)
        nav.item.navigate(TechnicalRecordKey(record.text))
        assertEquals(TechnicalRecordKey(record.text), nav.top)
        nav.item.shareFile("proof.json", "application/json", byteArrayOf(1, 2))
        assertEquals("file:proof.json:application/json:2", outside.calls.last())
    }

    @Test
    fun checkNavigationOpensProofsAndCaseLinks() {
        at(HomeKey.Home, CheckKey)
        nav.check.openProof(record)
        assertEquals(ProofPageKey(record.text), nav.top)
        val caseId = CaseId.of(1, ByteArray(CaseId.RANDOM_SIZE) { it.toByte() })
        nav.check.openCase(caseId, "k=abc")
        val top = nav.top as CaseLinkKey
        assertEquals(caseId.text, top.caseId)
        assertEquals("k=abc", top.fragment)
        nav.check.share("t")
        assertEquals(listOf("share:t"), outside.calls)
    }

    @Test
    fun webProofNavigationSelectsTheCheckTab() {
        at(HomeKey.Home, CheckKey, ProofPageKey(record.text))
        nav.proof.openCheck()
        assertEquals(listOf(HomeKey.Home, CheckKey), stack)
    }

    @Test
    fun topIsNullOnEmptyStack() {
        stack.clear()
        assertNull(nav.top)
    }
}
