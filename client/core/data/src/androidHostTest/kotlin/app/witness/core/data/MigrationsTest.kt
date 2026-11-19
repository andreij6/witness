package app.witness.core.data

import androidx.sqlite.driver.AndroidSQLiteDriver
import androidx.sqlite.execSQL
import kotlinx.coroutines.test.runTest
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], manifest = Config.NONE)
class MigrationsTest {
    @Test
    fun v1OutboxGainsANullableFingerprintsColumnAndKeepsRows() = runTest {
        val connection = AndroidSQLiteDriver().open(":memory:")
        connection.execSQL(V1_OUTBOX)
        connection.execSQL("INSERT INTO outbox VALUES ('r', 'k', 3, 1, 9, x'01', x'02', x'03', 0, 0, 'PENDING', NULL)")
        WitnessMigrations.V1_TO_V2.migrate(connection)
        val row = connection.prepare("SELECT counter, fingerprints FROM outbox").use { s ->
            assertTrue(s.step())
            s.getLong(0) to s.isNull(1)
        }
        assertEquals(3L to true, row)
        connection.close()
    }

    @Test
    fun v2CasesGainASummarySourceAndExistingSummariesCountAsHandWritten() = runTest {
        val connection = AndroidSQLiteDriver().open(":memory:")
        connection.execSQL(V2_CASE)
        connection.execSQL("INSERT INTO `case` VALUES (1, 'Leak', 'general', 1, 2, 'Water under sink', NULL)")
        connection.execSQL("INSERT INTO `case` VALUES (2, 'Crash', 'car_accident', 3, 4, NULL, NULL)")
        WitnessMigrations.V2_TO_V3.migrate(connection)
        val rows = connection.prepare("SELECT id, summary, summarySource FROM `case` ORDER BY id").use { s ->
            buildList {
                while (s.step()) add(Triple(s.getLong(0), if (s.isNull(1)) null else s.getText(1), if (s.isNull(2)) null else s.getText(2)))
            }
        }
        assertEquals(listOf(Triple(1L, "Water under sink", "WRITTEN"), Triple(2L, null, null)), rows)
        connection.close()
    }

    @Test
    fun migrationsCoverEveryVersionStep() {
        val steps = WitnessMigrations.ALL.map { it.startVersion to it.endVersion }
        assertEquals((1 until WitnessDatabase.VERSION).map { it to it + 1 }, steps)
    }

    private companion object {
        const val V2_CASE = "CREATE TABLE `case` (`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, `title` TEXT NOT NULL, `templateId` TEXT NOT NULL, " +
            "`createdAtMs` INTEGER NOT NULL, `updatedAtMs` INTEGER NOT NULL, `summary` TEXT, `shares` BLOB)"
        const val V1_OUTBOX = "CREATE TABLE outbox (`recordId` TEXT NOT NULL, `keyId` TEXT NOT NULL, `counter` INTEGER NOT NULL, " +
            "`isOriginal` INTEGER NOT NULL, `wallMs` INTEGER NOT NULL, `claimCbor` BLOB NOT NULL, `c2paClaimCbor` BLOB NOT NULL, " +
            "`coseSign1` BLOB NOT NULL, `attempts` INTEGER NOT NULL, `notBeforeMs` INTEGER NOT NULL, `state` TEXT NOT NULL, " +
            "`lastError` TEXT, PRIMARY KEY(`recordId`))"
    }
}
