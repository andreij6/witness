package app.witness.core.data

import android.content.Context
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import java.io.File

class DataGraph(context: Context) {
    private val app = context.applicationContext
    val database: WitnessDatabase by lazy { DatabaseFactory.create(app) }
    val images: ImageFiles by lazy { ImageFiles(File(app.filesDir, ImageFiles.DIR)) }
    val cipher: BlobCipher by lazy { AesGcmBlobCipher.keystore() }
    val captures: RoomCaptureStore by lazy { RoomCaptureStore(database, cipher, images) }
    val outbox: RoomOutboxStore by lazy { RoomOutboxStore(database) }
    val cases: RoomCaseStore by lazy { RoomCaseStore(database) }
    val counters: RoomCounterStore by lazy { RoomCounterStore(database) }
    val checkHistory: CheckHistoryStore by lazy { CheckHistoryStore(database) }
    val settings: DataStoreSettings by lazy {
        DataStoreSettings(PreferenceDataStoreFactory.create { File(app.filesDir, "datastore/${DataStoreSettings.FILE_NAME}.preferences_pb") })
    }
    val anchorScheduler: WorkManagerAnchorScheduler by lazy { WorkManagerAnchorScheduler.from(app) }
}
