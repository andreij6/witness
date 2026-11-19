package app.witness.core.data

import androidx.room3.ColumnInfo
import androidx.room3.Embedded
import androidx.room3.Entity
import androidx.room3.ForeignKey
import androidx.room3.Index
import androidx.room3.PrimaryKey
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.LocationConfidence
import app.witness.core.model.OutboxState
import app.witness.core.model.Verdict

@Entity(tableName = "capture", indices = [Index("createdAtMs"), Index("status")])
data class CaptureEntity(
    @PrimaryKey val recordId: String,
    val captureId: Long?,
    val origin: CaptureOrigin,
    val status: AnchoringStatus,
    val createdAtMs: Long,
    val keyId: String?,
    val counter: Long?,
    val locationConfidence: LocationConfidence,
    val width: Int,
    val height: Int,
    val anchorIx: Long?,
    val anchoredAtMs: Long?,
    val label: String?,
    val place: String?,
    val caption: String?,
    val altText: String?,
    val publicErased: Boolean,
)

@Entity(
    tableName = "derivative",
    foreignKeys = [ForeignKey(CaptureEntity::class, ["recordId"], ["recordId"], onDelete = ForeignKey.CASCADE, onUpdate = ForeignKey.CASCADE)],
    indices = [Index("parent")],
)
data class DerivativeEntity(@PrimaryKey val recordId: String, val parent: String, val edits: ByteArray)

@Entity(tableName = "case", indices = [Index("updatedAtMs")])
data class CaseEntity(
    @PrimaryKey(autoGenerate = true) val id: Long,
    val title: String,
    val templateId: String,
    val createdAtMs: Long,
    val updatedAtMs: Long,
    val summary: String?,
    val shares: ByteArray?,
    val summarySource: String? = null,
)

@Entity(
    tableName = "case_item",
    foreignKeys = [
        ForeignKey(CaseEntity::class, ["id"], ["caseId"], onDelete = ForeignKey.CASCADE),
        ForeignKey(CaptureEntity::class, ["recordId"], ["recordId"], onDelete = ForeignKey.CASCADE, onUpdate = ForeignKey.CASCADE),
    ],
    indices = [Index("caseId")],
)
data class CaseItemEntity(@PrimaryKey val recordId: String, val caseId: Long)

@Entity(
    tableName = "outbox",
    foreignKeys = [ForeignKey(CaptureEntity::class, ["recordId"], ["recordId"], onDelete = ForeignKey.CASCADE, onUpdate = ForeignKey.CASCADE)],
    indices = [Index("keyId", "counter", unique = true)],
)
data class OutboxEntity(
    @PrimaryKey val recordId: String,
    val keyId: String,
    val counter: Long,
    val isOriginal: Boolean,
    val wallMs: Long,
    val claimCbor: ByteArray,
    val c2paClaimCbor: ByteArray,
    val coseSign1: ByteArray,
    val attempts: Int,
    val notBeforeMs: Long,
    val state: OutboxState,
    val lastError: String?,
    val fingerprints: ByteArray? = null,
)

@Entity(tableName = "check_history", indices = [Index("checkedAtMs")])
data class CheckHistoryEntity(
    @PrimaryKey(autoGenerate = true) val id: Long,
    val checkedAtMs: Long,
    val verdict: Verdict,
    val recordId: String?,
    val label: String?,
)

@Entity(
    tableName = "loc_opening",
    foreignKeys = [ForeignKey(CaptureEntity::class, ["recordId"], ["recordId"], onDelete = ForeignKey.CASCADE, onUpdate = ForeignKey.CASCADE)],
)
data class LocOpeningEntity(@PrimaryKey val recordId: String, @ColumnInfo(name = "sealed") val sealed: ByteArray)

@Entity(tableName = "key_counter")
data class KeyCounterEntity(@PrimaryKey val keyId: String, val next: Long)

data class CaptureRow(
    @Embedded val capture: CaptureEntity,
    val caseId: Long?,
    val parent: String?,
    val edits: ByteArray?,
)
