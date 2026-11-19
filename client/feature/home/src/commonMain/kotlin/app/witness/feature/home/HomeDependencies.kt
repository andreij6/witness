package app.witness.feature.home

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId

class HomeDependencies(
    val vault: KeyVault,
    val captures: CaptureStore,
    val cases: CaseStore,
    val outbox: OutboxStore,
    val enrollment: Enrollment,
    val rotation: KeyRotation,
    val scheduler: AnchorScheduler,
    val thumbnails: ThumbnailSource,
    val clock: Clock,
    val zoneOffset: (Long) -> Int = ::systemZoneOffset,
) {
    val time: TimeLabels by lazy { TimeLabels(clock, zoneOffset) }
}

class HomeNavigator(
    val push: (HomeKey) -> Unit,
    val resetTo: (HomeKey) -> Unit,
    val back: () -> Unit,
    val openCamera: () -> Unit,
    val openCheck: () -> Unit,
    val openSettings: () -> Unit,
    val openItem: (RecordId) -> Unit,
    val openCase: (LocalCaseId) -> Unit,
    val newCase: () -> Unit,
) {
    fun finishOnboarding(openCamera: Boolean) {
        resetTo(HomeKey.Home)
        if (openCamera) openCamera()
    }
}

expect fun systemZoneOffset(epochMs: Long): Int
