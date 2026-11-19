package app.witness.core.data

import android.content.Context
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.AnchorReport

interface AnchorOutboxHost {
    val anchorOutbox: AnchorOutbox
}

class AnchorWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val host = applicationContext as? AnchorOutboxHost ?: return Result.failure()
        return resultFor(host.anchorOutbox(), runAttemptCount)
    }

    companion object {
        const val UNIQUE_NAME = "anchor"
        const val MAX_ATTEMPTS = 50
        const val BACKOFF_SECONDS = 30L

        fun resultFor(report: AnchorReport, attempt: Int): Result = when {
            !report.shouldRetry -> Result.success()
            attempt >= MAX_ATTEMPTS -> Result.failure()
            else -> Result.retry()
        }

        fun request(): OneTimeWorkRequest = OneTimeWorkRequestBuilder<AnchorWorker>()
            .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, BACKOFF_SECONDS, TimeUnit.SECONDS)
            .addTag(UNIQUE_NAME)
            .build()
    }
}

class WorkManagerAnchorScheduler(private val workManager: () -> WorkManager) : AnchorScheduler {
    override fun requestAnchoring() {
        workManager().enqueueUniqueWork(AnchorWorker.UNIQUE_NAME, ExistingWorkPolicy.APPEND_OR_REPLACE, AnchorWorker.request())
    }

    companion object {
        fun from(context: Context) = WorkManagerAnchorScheduler { WorkManager.getInstance(context.applicationContext) }
    }
}
