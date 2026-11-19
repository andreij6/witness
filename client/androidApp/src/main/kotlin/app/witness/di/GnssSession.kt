package app.witness.di

import java.util.concurrent.Executor

class GnssSession(private val start: () -> Unit, private val stop: () -> Unit, private val executor: Executor) {
    private var open = 0

    @Synchronized
    fun open() {
        if (open++ == 0) executor.execute(start)
    }

    @Synchronized
    fun close() {
        if (open == 0) return
        if (--open == 0) executor.execute(stop)
    }
}
