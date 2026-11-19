package app.witness.di

import kotlin.concurrent.Volatile

object AppForeground {
    @Volatile
    private var started = 0

    val isForeground: Boolean get() = started > 0

    @Synchronized
    fun onStart() {
        started++
    }

    @Synchronized
    fun onStop() {
        if (started > 0) started--
    }
}
