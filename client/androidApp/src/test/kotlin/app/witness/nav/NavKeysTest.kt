package app.witness.nav

import androidx.navigation3.runtime.NavKey
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
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
import kotlinx.serialization.PolymorphicSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NavKeysTest {
    private val json = Json { serializersModule = NavKeys.module }
    private val serializer = PolymorphicSerializer(NavKey::class)
    private val id = RecordId.fromDigest(ByteArray(32) { it.toByte() }).text

    private val every: List<NavKey> = HomeKey.all + listOf(
        SettingsKey.Root, SettingsKey.Plus, SettingsKey.LinkIdentity, SettingsKey.PrivacyData,
        CameraKey, PostComposerKey(id), PostSharedKey(id, LocationPrecision.NEIGHBORHOOD),
        CasesKey, CaseKey(9), CaseSwitcherKey, NewCaseKey, ShareCaseKey(9), ExportCaseKey(9),
        CheckKey, CheckSharedKey("a1"),
        ItemProofKey(id), TechnicalRecordKey(id), ProveLocationKey(id), ProofPageKey(id), ReportKey("p/$id"),
    )

    private fun roundTrip(key: NavKey): NavKey = json.decodeFromString(serializer, json.encodeToString(serializer, key))

    @Test
    fun everyAppKeyRoundTrips() {
        every.forEach { assertEquals(it, roundTrip(it), it.toString()) }
    }

    @Test
    fun everyKeyHasADistinctSerialName() {
        val names = every.map { json.encodeToString(serializer, it).substringAfter("\"type\":\"").substringBefore('"') }
        assertEquals(names.size, names.toSet().size)
    }

    @Test
    fun caseLinkKeyNeverPersistsItsKey() {
        val encoded = json.encodeToString(serializer, CaseLinkKey("CASE", "k=secret"))
        assertFalse("secret" in encoded)
        val back = roundTrip(CaseLinkKey("CASE", "k=secret")) as CaseLinkKey
        assertEquals("CASE", back.caseId)
        assertEquals("", back.fragment)
    }

    @Test
    fun unknownKeyTypeIsRejected() {
        assertFailsWith<SerializationException> { json.decodeFromString(serializer, """{"type":"evil.Key"}""") }
    }

    @Test
    fun configurationKnowsEveryKey() {
        every.forEach { assertTrue(NavKeys.configuration.serializersModule.getPolymorphic(NavKey::class, it) != null, it.toString()) }
    }
}
