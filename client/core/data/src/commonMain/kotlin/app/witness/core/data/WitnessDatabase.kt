package app.witness.core.data

import androidx.room3.ConstructedBy
import androidx.room3.Database
import androidx.room3.RoomDatabase
import androidx.room3.RoomDatabaseConstructor
import androidx.room3.immediateTransaction
import androidx.room3.migration.Migration
import androidx.sqlite.execSQL
import androidx.room3.useWriterConnection
import app.witness.core.domain.port.CounterStore
import app.witness.core.model.KeyId

@Database(
    entities = [
        CaptureEntity::class,
        DerivativeEntity::class,
        CaseEntity::class,
        CaseItemEntity::class,
        OutboxEntity::class,
        CheckHistoryEntity::class,
        LocOpeningEntity::class,
        KeyCounterEntity::class,
    ],
    version = WitnessDatabase.VERSION,
)
@ConstructedBy(WitnessDatabaseConstructor::class)
abstract class WitnessDatabase : RoomDatabase() {
    abstract fun captureDao(): CaptureDao
    abstract fun caseDao(): CaseDao
    abstract fun checkHistoryDao(): CheckHistoryDao
    abstract fun outboxDao(): OutboxDao
    abstract fun keyCounterDao(): KeyCounterDao

    companion object {
        const val FILE_NAME = "witness.db"
        const val VERSION = 3
    }
}

@Suppress("KotlinNoActualForExpect")
expect object WitnessDatabaseConstructor : RoomDatabaseConstructor<WitnessDatabase> {
    override fun initialize(): WitnessDatabase
}

object WitnessMigrations {
    const val OUTBOX_FINGERPRINTS = "ALTER TABLE outbox ADD COLUMN fingerprints BLOB"

    const val CASE_SUMMARY_SOURCE = "ALTER TABLE `case` ADD COLUMN summarySource TEXT"
    const val HAND_WRITTEN_SUMMARIES = "UPDATE `case` SET summarySource = 'WRITTEN' WHERE summary IS NOT NULL"

    val V1_TO_V2: Migration = Migration(1, 2) { connection -> connection.execSQL(OUTBOX_FINGERPRINTS) }

    val V2_TO_V3: Migration = Migration(2, 3) { connection ->
        connection.execSQL(CASE_SUMMARY_SOURCE)
        connection.execSQL(HAND_WRITTEN_SUMMARIES)
    }

    val ALL: Array<Migration> = arrayOf(V1_TO_V2, V2_TO_V3)
}

class RoomCounterStore(private val db: WitnessDatabase) : CounterStore {
    override suspend fun reserve(keyId: KeyId): Long = db.useWriterConnection { connection ->
        connection.immediateTransaction {
            usePrepared(SEED) { it.bindText(1, keyId.text); it.step() }
            usePrepared(INCREMENT) { it.bindText(1, keyId.text); it.step() }
            usePrepared(READ) {
                it.bindText(1, keyId.text)
                check(it.step()) { "counter row missing for key" }
                it.getLong(0)
            }
        }
    }

    private companion object {
        const val SEED = "INSERT OR IGNORE INTO key_counter (keyId, next) VALUES (?, 0)"
        const val INCREMENT = "UPDATE key_counter SET next = next + 1 WHERE keyId = ?"
        const val READ = "SELECT next - 1 FROM key_counter WHERE keyId = ?"
    }
}
