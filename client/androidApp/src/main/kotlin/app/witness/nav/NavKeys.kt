package app.witness.nav

import androidx.navigation3.runtime.NavKey
import androidx.savedstate.serialization.SavedStateConfiguration
import app.witness.feature.capture.CameraKey
import app.witness.feature.cases.CaseKey
import app.witness.feature.cases.CaseSwitcherKey
import app.witness.feature.cases.CasesKey
import app.witness.feature.cases.NewCaseKey
import app.witness.feature.check.CheckKey
import app.witness.feature.check.CheckSharedKey
import app.witness.feature.home.HomeKey
import app.witness.feature.post.PostComposerKey
import app.witness.feature.post.PostSharedKey
import app.witness.feature.proof.CaseLinkKey
import app.witness.feature.proof.ItemProofKey
import app.witness.feature.proof.ProofPageKey
import app.witness.feature.proof.ProveLocationKey
import app.witness.feature.proof.ReportKey
import app.witness.feature.proof.TechnicalRecordKey
import app.witness.feature.settings.SettingsKey
import app.witness.feature.share.ExportCaseKey
import app.witness.feature.share.ShareCaseKey
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.modules.SerializersModule
import kotlinx.serialization.modules.polymorphic

object NavKeys {
    @OptIn(ExperimentalSerializationApi::class)
    val module: SerializersModule = SerializersModule {
        polymorphic(NavKey::class) {
            subclassesOfSealed(HomeKey.serializer())
            subclassesOfSealed(SettingsKey.serializer())
            subclass(CameraKey::class, CameraKey.serializer())
            subclass(PostComposerKey::class, PostComposerKey.serializer())
            subclass(PostSharedKey::class, PostSharedKey.serializer())
            subclass(CasesKey::class, CasesKey.serializer())
            subclass(CaseKey::class, CaseKey.serializer())
            subclass(CaseSwitcherKey::class, CaseSwitcherKey.serializer())
            subclass(NewCaseKey::class, NewCaseKey.serializer())
            subclass(ShareCaseKey::class, ShareCaseKey.serializer())
            subclass(ExportCaseKey::class, ExportCaseKey.serializer())
            subclass(CheckKey::class, CheckKey.serializer())
            subclass(CheckSharedKey::class, CheckSharedKey.serializer())
            subclass(ItemProofKey::class, ItemProofKey.serializer())
            subclass(TechnicalRecordKey::class, TechnicalRecordKey.serializer())
            subclass(ProveLocationKey::class, ProveLocationKey.serializer())
            subclass(ProofPageKey::class, ProofPageKey.serializer())
            subclass(CaseLinkKey::class, CaseLinkKey.serializer())
            subclass(ReportKey::class, ReportKey.serializer())
        }
    }

    val configuration: SavedStateConfiguration = SavedStateConfiguration { serializersModule = module }
}
