package app.witness.feature.home

import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.SecurityLevel

const val SEALED_PHOTO = "Sealed photo"
const val IMPORTED_PHOTO = "Imported photo"
const val IMPORTED_FROM_GALLERY = "Imported from gallery"

fun Capture.witnessState(): WitnessState = when {
    origin == CaptureOrigin.IMPORT || status == AnchoringStatus.UNVERIFIED -> WitnessState.Unverified
    status == AnchoringStatus.ANCHORED -> WitnessState.Anchored
    status == AnchoringStatus.QUEUED -> WitnessState.Queued
    else -> WitnessState.Sealed
}

fun Capture.fallbackTitle(): String = if (origin == CaptureOrigin.IMPORT) IMPORTED_PHOTO else SEALED_PHOTO

fun Capture.headline(): String = caption.clean() ?: label.clean() ?: fallbackTitle()

fun Capture.shortTitle(): String = label.clean() ?: caption.clean() ?: fallbackTitle()

fun photoLabel(number: Int?): String? = number?.let { "Photo $it" }

fun SecurityLevel.label(): String = when (this) {
    SecurityLevel.STRONG_BOX -> "StrongBox"
    SecurityLevel.TEE -> "TEE"
    SecurityLevel.DEV -> "Dev key"
}

private fun String?.clean(): String? = this?.trim()?.takeIf { it.isNotEmpty() }
