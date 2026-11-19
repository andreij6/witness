package app.witness.di

import java.util.concurrent.Executor
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class GnssSessionTest {
    private val calls = ArrayList<String>()
    private val queued = ArrayList<Runnable>()
    private val deferred = Executor { queued += it }
    private val direct = Executor { it.run() }

    private fun session(executor: Executor = direct) = GnssSession({ calls += "start" }, { calls += "stop" }, executor)

    @Test
    fun openAndCloseStartAndStopOnce() {
        val s = session()
        s.open()
        s.close()
        assertEquals(listOf("start", "stop"), calls)
    }

    @Test
    fun nestedOpensShareOneRegistration() {
        val s = session()
        s.open()
        s.open()
        s.close()
        assertEquals(listOf("start"), calls)
        s.close()
        assertEquals(listOf("start", "stop"), calls)
    }

    @Test
    fun closeWithoutOpenIsIgnored() {
        val s = session()
        s.close()
        s.close()
        assertTrue(calls.isEmpty())
        s.open()
        assertEquals(listOf("start"), calls)
    }

    @Test
    fun workRunsOnTheExecutorInOrder() {
        val s = session(deferred)
        s.open()
        s.close()
        s.open()
        assertTrue(calls.isEmpty())
        queued.forEach(Runnable::run)
        assertEquals(listOf("start", "stop", "start"), calls)
    }
}
