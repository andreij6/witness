package app.witness.core.data

import androidx.work.BackoffPolicy
import androidx.work.ListenableWorker
import androidx.work.NetworkType
import androidx.work.OutOfQuotaPolicy
import app.witness.core.domain.usecase.AnchorReport
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AnchorWorkerTest {
    @Test
    fun cleanRunSucceeds() {
        assertTrue(AnchorWorker.resultFor(AnchorReport(anchored = 3, failed = 1), 0) is ListenableWorker.Result.Success)
        assertTrue(AnchorWorker.resultFor(AnchorReport(needsRotation = true), 0) is ListenableWorker.Result.Success)
    }

    @Test
    fun pendingWorkRetries() {
        listOf(AnchorReport(deferred = 1), AnchorReport(offline = true)).forEach {
            assertTrue(AnchorWorker.resultFor(it, 1) is ListenableWorker.Result.Retry)
        }
    }

    @Test
    fun givesUpAfterMaxAttempts() {
        assertTrue(AnchorWorker.resultFor(AnchorReport(offline = true), AnchorWorker.MAX_ATTEMPTS) is ListenableWorker.Result.Failure)
    }

    @Test
    fun requestIsExpeditedNetworkBoundExponential() {
        val spec = AnchorWorker.request().workSpec
        assertTrue(spec.expedited)
        assertEquals(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST, spec.outOfQuotaPolicy)
        assertEquals(NetworkType.CONNECTED, spec.constraints.requiredNetworkType)
        assertEquals(BackoffPolicy.EXPONENTIAL, spec.backoffPolicy)
        assertEquals(30_000, spec.backoffDelayDuration)
        assertEquals(AnchorWorker::class.java.name, spec.workerClassName)
    }
}
