package app.witness.feature.check

import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.domain.usecase.Geohash
import app.witness.core.model.BlurClass
import app.witness.core.model.CheckResult
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EditClass
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Reference
import app.witness.core.model.Region
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import app.witness.core.ui.FactBadge
import app.witness.core.ui.FactStatus
import app.witness.core.ui.Formatters
import app.witness.core.ui.NormRect
import app.witness.core.ui.ProofFact
import app.witness.core.ui.headline
import app.witness.core.ui.qualifier
import app.witness.core.ui.summary
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList

@Immutable
data class ChangeLine(val icon: WitnessIcon, val tone: Tone, val title: String, val detail: String?, val by: String? = null)

@Immutable
data class CheckOutcome(
    val verdict: Verdict,
    val headline: String,
    val qualifier: String?,
    val body: String,
    val note: String? = null,
    val changes: ImmutableList<ChangeLine> = persistentListOf(),
    val facts: ImmutableList<ProofFact> = persistentListOf(),
    val captured: String? = null,
    val recordId: RecordId? = null,
    val referenceCaption: String? = null,
    val referenceDate: String? = null,
    val referenceAspect: Float? = null,
    val crop: NormRect? = null,
    val regions: ImmutableList<NormRect> = persistentListOf(),
    val matchedBy: String? = null,
    val aligned: String? = null,
    val registryUnverified: Boolean = false,
) {
    val hasReference: Boolean get() = recordId != null && verdict != Verdict.NO_RECORD

    companion object {
        private const val ONE = Region.Q16_ONE

        fun of(result: CheckResult, input: Dims?, time: TimeText): CheckOutcome {
            val ref = result.reference.takeIf { result.verdict != Verdict.NO_RECORD }
            val noPublic = VerdictFlag.NO_PUBLIC_REFERENCE in result.flags && result.verdict == Verdict.MATCHES_WITH_CHANGES
            val claimDims = ref?.claim?.dims?.takeIf { it.isValid }
            val refAspect = claimDims?.let { it.width.toFloat() / it.height }
            val inputAspect = input?.takeIf { it.isValid }?.let { it.width.toFloat() / it.height }
            val compared = ref?.comparedQ16 ?: ONE
            val crop = if (refAspect != null && inputAspect != null && compared < ONE && kotlin.math.abs(refAspect - inputAspect) > ASPECT_EPSILON) {
                NormRect.centeredCrop(refAspect, inputAspect)
            } else null
            return CheckOutcome(
                verdict = result.verdict,
                headline = when {
                    result.registryUnverified -> UNVERIFIED_HEADLINE
                    noPublic -> "Matches a sealed photo"
                    else -> result.verdict.headline()
                },
                qualifier = if (noPublic) "with no public reference" else result.verdict.qualifier(),
                body = body(result, noPublic),
                note = note(result),
                changes = changes(result, ref, input).toImmutableList(),
                facts = facts(result, ref, time).toImmutableList(),
                captured = ref?.let { captured(it, result, time) },
                recordId = ref?.recordId,
                referenceCaption = ref?.let { referenceCaption(result.declared, it, time) },
                referenceDate = ref?.let { time.date(it.claim.time.wallMs) },
                referenceAspect = refAspect,
                crop = crop,
                regions = result.regions.map(::norm).toImmutableList(),
                matchedBy = ref?.let { matchedBy(it.evidence, result) },
                aligned = ref?.let { "${percent(it.comparedQ16)}% of reference frame" },
                registryUnverified = result.registryUnverified,
            )
        }

        fun body(result: CheckResult, noPublic: Boolean): String {
            val regions = result.regions.size
            return when {
                result.registryUnverified -> UNVERIFIED_BODY
                noPublic -> "This copy matches a sealed capture that was never posted, so there's no reference image to compare. The capture facts are shown instead."
                result.verdict == Verdict.ALTERED && regions > 1 ->
                    "This copy matches a sealed capture, but $regions regions were changed. The sealed reference is authentic — this copy is not."
                result.verdict == Verdict.ALTERED && regions == 0 && VerdictReason.SEAL_BROKEN in result.reasons ->
                    "This file's seal is broken — it was changed after it was sealed."
                else -> result.verdict.summary()
            }
        }

        fun note(result: CheckResult): String? {
            val parts = ArrayList<String>(4)
            if (result.proofsRejected && !result.registryUnverified) parts += "Some registry answers failed verification and were ignored."
            if (VerdictReason.FOREIGN_WATERMARK in result.reasons) parts += "It carries a watermark from a different photo, so no reference is shown."
            if (VerdictFlag.COULDNT_COMPARE in result.flags) parts += "Some regions couldn't be compared."
            if (VerdictFlag.KEY_REVOKED_AFTER_ANCHOR in result.flags) parts += "The phone's key was revoked after this was anchored."
            if (VerdictFlag.LATE_ANCHOR in result.flags) parts += "It was anchored more than a day after capture."
            if (VerdictFlag.DEV_ATTESTATION in result.flags) parts += "Captured on a development device."
            if (VerdictFlag.TRUNCATED_CANDIDATES in result.flags) parts += "Not every index answered — check again for a complete search."
            return parts.takeIf { it.isNotEmpty() }?.joinToString(" ")
        }

        fun changes(result: CheckResult, ref: Reference?, input: Dims?): List<ChangeLine> {
            if (ref == null || result.verdict == Verdict.NO_RECORD || result.verdict == Verdict.SEALED_UNANCHORED) return emptyList()
            val out = ArrayList<ChangeLine>(5)
            if (result.verdict == Verdict.ALTERED && result.regions.isNotEmpty()) {
                out += ChangeLine(WitnessIcon.Error, Tone.Crit, "Changed in ${Formatters.plural(result.regions.size, "region")}", "Highlighted on this copy", "This copy")
            }
            val removed = 100 - percent(ref.comparedQ16)
            if (removed >= 1) {
                val square = input != null && input.isValid && input.width == input.height
                out += ChangeLine(WitnessIcon.Crop, Tone.Edit, if (square) "Cropped to a square" else "Cropped", "About $removed% of the frame removed", "This copy")
            }
            if (ref.evidence == MatchEvidence.PIXELS || ref.evidence == MatchEvidence.GRID) {
                out += ChangeLine(WitnessIcon.Image, Tone.Edit, "Recompressed", "${100 - percent(ref.changedQ16)}% of the compared area matches", "This copy")
            }
            val blurs = result.declared.filterIsInstance<Edit.Blur>()
            if (blurs.isNotEmpty()) {
                out += ChangeLine(WitnessIcon.Edit, Tone.Edit, "${Formatters.plural(blurs.size, "blur")} declared", blurNouns(blurs), "Declared by sender")
            }
            val global = result.declared.filter { it.editClass == EditClass.GLOBAL }
            if (global.isNotEmpty()) {
                out += ChangeLine(WitnessIcon.Tune, Tone.Edit, "Declared edits", global.map { editNoun(it) }.distinct().joinToString(", ").replaceFirstChar { it.uppercase() }, "Declared by sender")
            }
            if (VerdictFlag.COULDNT_COMPARE in result.flags) {
                out += ChangeLine(WitnessIcon.Help, Tone.Muted, "Some regions couldn't be compared", "Heavy crop, low resolution or overlays")
            }
            return out
        }

        fun facts(result: CheckResult, ref: Reference?, time: TimeText): List<ProofFact> = when (result.verdict) {
            Verdict.NO_RECORD -> checked(result)
            Verdict.SEALED_UNANCHORED -> listOf(
                ProofFact("Seal", "Valid · unchanged since sealing", WitnessIcon.Lock, badge = FactBadge("Valid", Tone.Ok, WitnessIcon.Check)),
                ProofFact("Device", device(ref, result), WitnessIcon.PhoneAndroid, badge = FactBadge("Genuine", Tone.Ok, WitnessIcon.Check)),
                ProofFact("Anchor", "Not in the registry yet", WitnessIcon.Verified, badge = FactBadge("Pending", Tone.Warn, WitnessIcon.HourglassTop)),
            )
            else -> buildList {
                if (ref == null) return@buildList
                if (result.verdict == Verdict.ALTERED) {
                    val n = result.regions.size
                    add(ProofFact("Content", if (n > 0) "Differs in ${Formatters.plural(n, "region")}" else "Seal broken", status = FactStatus.Failed))
                }
                add(ProofFact("Match", matchedBy(ref.evidence, result), status = FactStatus.Passed))
                add(ProofFact("Device", device(ref, result), status = FactStatus.Passed))
                ref.anchor?.let { add(ProofFact("Anchored", time.dateTime(it.anchoredAtMs), status = FactStatus.Passed)) }
            }
        }

        fun checked(result: CheckResult): List<ProofFact> {
            val r = result.reasons
            val watermark = when {
                VerdictReason.FOREIGN_WATERMARK in r -> "From a different photo"
                VerdictReason.WATERMARK_UNAVAILABLE in r -> "Reader unavailable here"
                VerdictReason.WATERMARK_FOUND in r -> "Found, not confirmed"
                else -> "None found"
            }
            val fingerprints = when {
                VerdictReason.UNDECODABLE in r -> "Couldn't read this file"
                VerdictReason.INDEX_PENDING in r -> "Index still catching up"
                VerdictReason.UNCONFIRMED in r -> "Similar photos, none confirmed"
                else -> "No match"
            }
            val credentials = when {
                VerdictReason.OTHER_CREDENTIALS in r -> "Content Credentials from another app"
                VerdictReason.UNVERIFIED_CREDENTIALS in r -> "Content Credentials couldn't be verified"
                VerdictReason.KEY_NOT_TRUSTED in r || VerdictReason.KEY_NOT_TRUSTED_AT_ANCHOR in r -> "Signed by a key that isn't trusted"
                else -> "No C2PA data in this file"
            }
            val checkedFacts = listOf(
                ProofFact("Watermark", watermark, WitnessIcon.Search),
                ProofFact("Fingerprints", fingerprints, WitnessIcon.Search),
                ProofFact("Credentials", credentials, WitnessIcon.Search),
            )
            return if (result.proofsRejected) listOf(ProofFact("Registry", "Answers failed verification", WitnessIcon.Error, FactStatus.Failed)) + checkedFacts else checkedFacts
        }

        fun matchedBy(evidence: MatchEvidence, result: CheckResult): String {
            val watermark = VerdictReason.WATERMARK_FOUND in result.reasons
            return when (evidence) {
                MatchEvidence.SEAL -> "Content Credentials"
                MatchEvidence.EXACT_HASH -> if (VerdictReason.EXACT_PIXEL_HASH in result.reasons) "Exact pixels" else "Exact file"
                MatchEvidence.PIXELS -> if (watermark) "Watermark + fingerprint" else "Fingerprint"
                MatchEvidence.GRID -> if (watermark) "Watermark + region grid" else "Region grid"
            }
        }

        fun isDev(ref: Reference?, result: CheckResult): Boolean =
            VerdictFlag.DEV_ATTESTATION in result.flags || ref?.claim?.checks?.keySecurity == SecurityLevel.DEV

        fun device(ref: Reference?, result: CheckResult): String =
            if (isDev(ref, result)) "Development device · not production" else "Genuine Android phone · attested"

        fun captured(ref: Reference, result: CheckResult, time: TimeText): String = Formatters.joinDot(
            "Captured ${time.dateTime(ref.claim.time.wallMs)}",
            ref.claim.coarse?.let { Geohash.area(it) ?: "area $it" },
            if (isDev(ref, result)) "development device" else "genuine Android phone",
        )

        fun referenceCaption(declared: List<Edit>, ref: Reference, time: TimeText): String {
            val blurs = declared.count { it is Edit.Blur }
            return if (blurs > 0) "With ${Formatters.declaredEdits(blurs, "blur")}" else time.date(ref.claim.time.wallMs) + " · " + time.time(ref.claim.time.wallMs)
        }

        fun blurNouns(blurs: List<Edit.Blur>): String {
            val counts = IntArray(BlurClass.entries.size)
            for (b in blurs) counts[b.blurClass.ordinal]++
            val nouns = BlurClass.entries.filter { counts[it.ordinal] > 0 }.map { c ->
                val n = counts[c.ordinal]
                val noun = BLUR_NOUNS[c.ordinal]
                if (n == 1) noun else "$n ${noun}s"
            }
            val text = when (nouns.size) {
                1 -> nouns[0]
                else -> nouns.dropLast(1).joinToString(", ") + " and " + nouns.last()
            }
            return text.replaceFirstChar { it.uppercase() }
        }

        fun editNoun(edit: Edit): String = when (edit) {
            is Edit.Crop -> "crop"
            is Edit.Rotate -> "straighten"
            is Edit.Tone -> "tone"
            is Edit.Filter -> "filter"
            is Edit.Blur -> "blur"
            is Edit.Resize -> "resize"
            is Edit.Recompress -> "recompress"
            is Edit.Orient -> "orientation"
        }

        fun norm(r: Region): NormRect {
            val l = (r.x.toFloat() / ONE).coerceIn(0f, 1f)
            val t = (r.y.toFloat() / ONE).coerceIn(0f, 1f)
            return NormRect(l, t, ((r.x.toLong() + r.w).toFloat() / ONE).coerceIn(l, 1f), ((r.y.toLong() + r.h).toFloat() / ONE).coerceIn(t, 1f))
        }

        fun percent(q16: Int): Int = ((q16.coerceIn(0, ONE).toLong() * 100 + ONE / 2) / ONE).toInt()

        private const val ASPECT_EPSILON = 0.01f
        const val UNVERIFIED_HEADLINE = "Couldn't verify the registry"
        const val UNVERIFIED_BODY =
            "The Witness registry sent answers that failed verification, so we can't say whether this photo has a record. Nothing was trusted. Try again later or on another network."
        private val BLUR_NOUNS = arrayOf("face", "plate", "house number", "area")
    }
}
