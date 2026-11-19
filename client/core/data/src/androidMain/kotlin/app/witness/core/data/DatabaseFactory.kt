package app.witness.core.data

import android.content.Context
import androidx.room3.Room
import androidx.sqlite.SQLiteDriver
import androidx.sqlite.driver.bundled.BundledSQLiteDriver
import kotlinx.coroutines.Dispatchers

object DatabaseFactory {
    fun create(context: Context, driver: SQLiteDriver = BundledSQLiteDriver()): WitnessDatabase {
        val app = context.applicationContext
        return Room.databaseBuilder<WitnessDatabase>(app, app.getDatabasePath(WitnessDatabase.FILE_NAME).absolutePath)
            .setDriver(driver)
            .setQueryCoroutineContext(Dispatchers.IO)
            .addMigrations(*WitnessMigrations.ALL)
            .build()
    }
}
