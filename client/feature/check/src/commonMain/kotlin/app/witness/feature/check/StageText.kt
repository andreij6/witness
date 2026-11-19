package app.witness.feature.check

import app.witness.core.model.CheckStage
import app.witness.core.model.StageState
import app.witness.core.model.StageStatus
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.toImmutableList
import app.witness.core.ui.CheckStage as UiStage
import app.witness.core.ui.StageStatus as UiStatus

enum class CheckSurface { App, Web }

object StageText {
    fun map(stages: List<StageStatus>, surface: CheckSurface): ImmutableList<UiStage> = stages.map { stage(it, surface) }.toImmutableList()

    fun stage(s: StageStatus, surface: CheckSurface): UiStage {
        val status = status(s.state)
        val web = surface == CheckSurface.Web
        val counted = s.total > 0
        val progress = if (counted && status == UiStatus.Running) (s.completed.toFloat() / s.total).coerceIn(0f, 1f) else null
        return when (s.stage) {
            CheckStage.READ_FILE -> UiStage(if (web) "Reading file" else "Reading the file", s.detail, status)
            CheckStage.CREDENTIALS -> UiStage(
                if (!web && status == UiStatus.Skipped) "Content Credentials · none found" else "Content Credentials",
                s.detail ?: if (status == UiStatus.Skipped) {
                    if (web) "None in this file" else "Stripped on upload — expected for screenshots"
                } else null,
                status,
            )
            CheckStage.FINGERPRINTS -> UiStage(
                if (web) "Fingerprints" else "Searching fingerprints",
                when {
                    counted && web && status == UiStatus.Running -> "Searching ${s.total} shards…"
                    counted -> "${s.total} index shards · ${s.completed} answered"
                    else -> s.detail
                },
                status,
                progress,
                if (web && progress != null) "${s.completed} of ${s.total}" else null,
            )
            CheckStage.WATERMARK -> UiStage(
                if (web) "Watermark" else "Reading the watermark",
                s.detail ?: if (web && status == UiStatus.Pending) "Loads only if needed" else null,
                status,
                progress,
            )
            CheckStage.COMPARE -> UiStage(
                if (web) "Comparing" else "Comparing with the reference",
                s.detail ?: if (web && status == UiStatus.Pending) "Waiting" else null,
                status,
                progress,
            )
        }
    }

    fun status(state: StageState): UiStatus = when (state) {
        StageState.WAITING -> UiStatus.Pending
        StageState.RUNNING -> UiStatus.Running
        StageState.DONE -> UiStatus.Done
        StageState.SKIPPED, StageState.FAILED -> UiStatus.Skipped
    }
}
