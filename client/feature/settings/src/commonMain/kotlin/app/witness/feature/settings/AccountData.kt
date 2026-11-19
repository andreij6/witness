package app.witness.feature.settings

import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.usecase.EraseReport

sealed interface ExportOutcome {
    data class Ready(val records: Int) : ExportOutcome

    data class Failed(val reason: String) : ExportOutcome
}

fun interface DataExporter {
    suspend fun export(): ExportOutcome
}

sealed interface DeleteOutcome {
    data class Deleted(val erased: Int) : DeleteOutcome

    data class Incomplete(val report: EraseReport) : DeleteOutcome
}

class DeleteAccount(private val eraseAll: suspend () -> EraseReport, private val identity: IdentityAccount) {
    suspend operator fun invoke(): DeleteOutcome {
        val report = eraseAll()
        if (!report.complete) return DeleteOutcome.Incomplete(report)
        return try {
            identity.unlink()
            DeleteOutcome.Deleted(report.erased.size)
        } catch (_: OfflineException) {
            DeleteOutcome.Incomplete(report.copy(offline = true))
        }
    }
}
