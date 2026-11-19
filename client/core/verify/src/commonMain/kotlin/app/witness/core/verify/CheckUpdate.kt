package app.witness.core.verify

enum class CheckStage { READ_FILE, CREDENTIALS, FINGERPRINTS, WATERMARK, COMPARE }

enum class StageState { WAITING, RUNNING, DONE, SKIPPED }

sealed interface CheckUpdate {
    data class Progress(val stage: CheckStage, val state: StageState, val answered: Int = 0, val total: Int = 0) : CheckUpdate

    data class Done(val judgement: Judgement) : CheckUpdate
}
