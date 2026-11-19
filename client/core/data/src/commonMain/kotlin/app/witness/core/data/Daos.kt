package app.witness.core.data

import androidx.room3.Dao
import androidx.room3.Delete
import androidx.room3.Insert
import androidx.room3.OnConflictStrategy
import androidx.room3.Query
import androidx.room3.Transaction
import androidx.room3.Update
import androidx.room3.Upsert
import app.witness.core.model.AnchoringStatus
import kotlinx.coroutines.flow.Flow

private const val CAPTURE_ROW =
    "SELECT c.*, i.caseId AS caseId, d.parent AS parent, d.edits AS edits FROM capture c " +
        "LEFT JOIN case_item i ON i.recordId = c.recordId LEFT JOIN derivative d ON d.recordId = c.recordId"

@Dao
interface CaptureDao {
    @Query("$CAPTURE_ROW ORDER BY c.createdAtMs DESC")
    fun observeAll(): Flow<List<CaptureRow>>

    @Query("$CAPTURE_ROW WHERE c.recordId = :recordId")
    suspend fun get(recordId: String): CaptureRow?

    @Query(
        "WITH RECURSIVE tree(id) AS (SELECT recordId FROM derivative WHERE parent = :recordId " +
            "UNION SELECT x.recordId FROM derivative x JOIN tree ON x.parent = tree.id) " +
            "$CAPTURE_ROW WHERE c.recordId IN (SELECT id FROM tree) ORDER BY c.createdAtMs",
    )
    suspend fun derivativesOf(recordId: String): List<CaptureRow>

    @Insert
    suspend fun insertCapture(capture: CaptureEntity)

    @Update
    suspend fun updateCapture(capture: CaptureEntity): Int

    @Upsert
    suspend fun upsertDerivative(derivative: DerivativeEntity)

    @Query("DELETE FROM derivative WHERE recordId = :recordId")
    suspend fun deleteDerivative(recordId: String)

    @Upsert
    suspend fun upsertCaseItem(item: CaseItemEntity)

    @Query("DELETE FROM case_item WHERE recordId = :recordId")
    suspend fun deleteCaseItem(recordId: String)

    @Insert
    suspend fun insertOutbox(entry: OutboxEntity)

    @Update
    suspend fun updateOutbox(entry: OutboxEntity): Int

    @Upsert
    suspend fun upsertOpening(opening: LocOpeningEntity)

    @Query("SELECT sealed FROM loc_opening WHERE recordId = :recordId")
    suspend fun opening(recordId: String): ByteArray?

    @Query("DELETE FROM capture WHERE recordId = :recordId")
    suspend fun delete(recordId: String)

    @Query("UPDATE capture SET recordId = :new WHERE recordId = :old")
    suspend fun rename(old: String, new: String): Int

    @Query("UPDATE derivative SET parent = :new WHERE parent = :old")
    suspend fun reparent(old: String, new: String)

    @Query(
        "UPDATE capture SET status = :status, anchorIx = COALESCE(:anchorIx, anchorIx), " +
            "anchoredAtMs = COALESCE(:anchoredAtMs, anchoredAtMs) WHERE recordId = :recordId",
    )
    suspend fun setStatus(recordId: String, status: AnchoringStatus, anchorIx: Long?, anchoredAtMs: Long?)

    @Transaction
    suspend fun insert(row: CaptureRow, outbox: OutboxEntity?, opening: LocOpeningEntity?) {
        insertCapture(row.capture)
        attach(row)
        if (outbox != null) insertOutbox(outbox)
        if (opening != null) upsertOpening(opening)
    }

    @Transaction
    suspend fun update(row: CaptureRow) {
        if (updateCapture(row.capture) == 0) return
        attach(row)
    }

    @Transaction
    suspend fun replace(old: String, row: CaptureRow, outbox: OutboxEntity) {
        val id = row.capture.recordId
        if (old != id) {
            check(rename(old, id) == 1) { "unknown capture" }
            reparent(old, id)
        }
        updateCapture(row.capture)
        attach(row)
        if (updateOutbox(outbox) == 0) insertOutbox(outbox)
    }

    private suspend fun attach(row: CaptureRow) {
        val id = row.capture.recordId
        if (row.caseId == null) deleteCaseItem(id) else upsertCaseItem(CaseItemEntity(id, row.caseId))
        if (row.parent == null) deleteDerivative(id) else upsertDerivative(DerivativeEntity(id, row.parent, row.edits ?: EMPTY))
    }

    private companion object {
        val EMPTY = ByteArray(0)
    }
}

@Dao
interface OutboxDao {
    @Query("SELECT * FROM outbox ORDER BY keyId, counter")
    fun observe(): Flow<List<OutboxEntity>>

    @Query("SELECT * FROM outbox ORDER BY keyId, counter")
    suspend fun all(): List<OutboxEntity>

    @Insert
    suspend fun insert(entry: OutboxEntity)

    @Update
    suspend fun update(entry: OutboxEntity): Int

    @Transaction
    suspend fun put(entry: OutboxEntity) {
        if (update(entry) == 0) insert(entry)
    }

    @Query("DELETE FROM outbox WHERE recordId = :recordId")
    suspend fun remove(recordId: String)
}

@Dao
interface CaseDao {
    @Query("SELECT * FROM `case` ORDER BY updatedAtMs DESC, id DESC")
    fun observeAll(): Flow<List<CaseEntity>>

    @Query("SELECT * FROM `case` WHERE id = :id")
    suspend fun get(id: Long): CaseEntity?

    @Insert
    suspend fun insert(case: CaseEntity): Long

    @Update
    suspend fun update(case: CaseEntity)

    @Query("DELETE FROM `case` WHERE id = :id")
    suspend fun delete(id: Long)
}

@Dao
interface CheckHistoryDao {
    @Insert
    suspend fun insert(check: CheckHistoryEntity): Long

    @Query("DELETE FROM check_history WHERE id NOT IN (SELECT id FROM check_history ORDER BY checkedAtMs DESC, id DESC LIMIT :keep)")
    suspend fun trim(keep: Int)

    @Transaction
    suspend fun record(check: CheckHistoryEntity, keep: Int) {
        insert(check)
        trim(keep)
    }

    @Query("SELECT * FROM check_history ORDER BY checkedAtMs DESC, id DESC LIMIT :limit")
    fun observeRecent(limit: Int): Flow<List<CheckHistoryEntity>>

    @Query("DELETE FROM check_history")
    suspend fun clear()
}

@Dao
interface KeyCounterDao {
    @Upsert
    suspend fun upsert(counter: KeyCounterEntity)

    @Query("SELECT next FROM key_counter WHERE keyId = :keyId")
    suspend fun next(keyId: String): Long?
}
