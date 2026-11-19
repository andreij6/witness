package app.witness.feature.settings

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlin.random.Random
import kotlin.test.AfterTest
import kotlin.test.BeforeTest

object TestPrincipals {
    fun selfAuthenticating(seed: Int): Principal {
        val bytes = Random(seed).nextBytes(Principal.MAX_BYTES)
        bytes[bytes.lastIndex] = 0x02
        return Principal.fromBytes(bytes)
    }

    val alice: Principal = selfAuthenticating(1)
    val bob: Principal = selfAuthenticating(2)
}

@OptIn(ExperimentalCoroutinesApi::class)
abstract class MainDispatcherTest {
    @BeforeTest
    fun setMainDispatcher() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun resetMainDispatcher() = Dispatchers.resetMain()
}

@OptIn(ExperimentalCoroutinesApi::class)
fun TestScope.keepActive(flow: Flow<*>) {
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { flow.collect {} }
}

@OptIn(ExperimentalCoroutinesApi::class)
fun <T> TestScope.record(flow: Flow<T>): List<T> {
    val out = ArrayList<T>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { flow.collect { out += it } }
    return out
}
