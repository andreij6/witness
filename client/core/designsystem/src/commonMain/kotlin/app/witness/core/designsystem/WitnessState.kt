package app.witness.core.designsystem

enum class WitnessState(val label: String, val shortLabel: String, val tone: Tone, val icon: WitnessIcon) {
    Original("Original", "Original", Tone.Ok, WitnessIcon.CheckCircle),
    Changes("Matches an original, with changes", "With changes", Tone.Edit, WitnessIcon.Compare),
    Altered("Altered", "Altered", Tone.Crit, WitnessIcon.Error),
    NoRecord("No Witness record", "No record", Tone.Muted, WitnessIcon.Help),
    Anchored("Anchored", "Anchored", Tone.Ok, WitnessIcon.Verified),
    Queued("Queued", "Queued", Tone.Warn, WitnessIcon.HourglassTop),
    Sealed("Sealed", "Sealed", Tone.Ink, WitnessIcon.Lock),
    Edited("Declared edit", "Edited", Tone.Edit, WitnessIcon.Edit),
    Unverified("Unverified import", "Unverified", Tone.Muted, WitnessIcon.Help),
    Unanchored("Sealed · not yet anchored", "Not yet anchored", Tone.Ink, WitnessIcon.Lock),
}
