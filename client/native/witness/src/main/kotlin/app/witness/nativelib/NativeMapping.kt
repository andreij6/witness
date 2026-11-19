package app.witness.nativelib

import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.ShardMap
import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CheckResult
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.ImageFormat
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.KeyRef
import app.witness.core.model.KeyStatus
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Provisioning
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.Reference
import app.witness.core.model.Region
import app.witness.core.model.SecurityLevel
import app.witness.core.model.StatusChange
import app.witness.core.model.Submission
import app.witness.core.model.TimeEvidence
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import app.witness.core.model.WitnessClaim
import uniffi.witness_android.Anchored
import uniffi.witness_android.CertifiedRecord
import uniffi.witness_android.ClaimView
import uniffi.witness_android.Kind
import uniffi.witness_android.LocationSpec
import uniffi.witness_android.Shards
import uniffi.witness_android.SubmitErr
import uniffi.witness_android.VerdictView
import uniffi.witness_android.BlurClass as NBlurClass
import uniffi.witness_android.BootState as NBootState
import uniffi.witness_android.Checks as NChecks
import uniffi.witness_android.Confidence as NConfidence
import uniffi.witness_android.Edit as NEdit
import uniffi.witness_android.Evidence as NEvidence
import uniffi.witness_android.Fresh as NFresh
import uniffi.witness_android.ImageFormat as NImageFormat
import uniffi.witness_android.KeyInfo as NKeyInfo
import uniffi.witness_android.KeyStatus as NKeyStatus
import uniffi.witness_android.Prov as NProv
import uniffi.witness_android.Rect as NRect
import uniffi.witness_android.Region as NRegion
import uniffi.witness_android.Security as NSecurity
import uniffi.witness_android.SecurityLevel as NSecurityLevel
import uniffi.witness_android.Submission as NSubmission
import uniffi.witness_android.SubmitResult as NSubmitResult
import uniffi.witness_android.TimeEvidence as NTimeEvidence

object NativeMapping {
    const val NS_PER_MS: Long = 1_000_000

    fun nsToMs(ns: ULong): Long = (ns / NS_PER_MS.toULong()).toLong()

    fun claim(view: ClaimView): WitnessClaim = WitnessClaim(
        captureId = captureId(view.captureId),
        kind = kind(view.kind),
        assetHash = ByteString.wrap(view.assetHash),
        pixelHash = ByteString.wrap(view.pixelHash),
        fpCommit = ByteString.wrap(view.fpCommit),
        dims = Dims(view.dims.width.toInt(), view.dims.height.toInt()),
        time = time(view.time),
        location = location(view.location),
        checks = checks(view.checks),
        keyRef = KeyRef(KeyId.fromBytes(view.keyId), view.counter.toLong()),
        certSerial = Hex.of(view.certSerial),
        coarse = view.coarse,
    )

    fun captureId(value: ULong): CaptureId = requireNotNull(CaptureId.fromLong(value.toLong())) { "capture id out of range" }

    fun kind(kind: Kind): ClaimKind = when (kind) {
        Kind.Original -> ClaimKind.Original
        is Kind.Derivative -> ClaimKind.Derivative(RecordId.parse(kind.parent), captureId(kind.root), kind.edits.map(::edit))
    }

    fun kind(kind: ClaimKind): Kind = when (kind) {
        ClaimKind.Original -> Kind.Original
        is ClaimKind.Derivative -> Kind.Derivative(kind.parent.text, kind.root.value.toULong(), kind.edits.map(::edit))
    }

    fun time(t: NTimeEvidence): TimeEvidence = TimeEvidence(t.wallMs.toLong(), t.gnssMs?.toLong(), t.elapsedNs.toLong(), t.bootCount.toInt())

    fun time(t: TimeEvidence): NTimeEvidence = NTimeEvidence(t.wallMs.toULong(), t.gnssMs?.toULong(), t.elapsedNs.toULong(), t.bootCount.toUInt())

    fun location(l: LocationSpec): LocationEvidence =
        LocationEvidence(l.commit?.let(ByteString::wrap), LocationConfidence.valueOf(l.confidence.name), EnumBits(l.reasons.toInt()))

    fun location(l: LocationEvidence): LocationSpec =
        LocationSpec(l.commit?.toByteArray(), NConfidence.valueOf(l.confidence.name), l.reasons.bits.toUShort())

    fun checks(c: NChecks): Checks = Checks(
        SecurityLevel.valueOf(c.keySecurity.name),
        BootState.valueOf(c.boot.name),
        c.deviceLocked,
        c.mockLocation,
        EnumBits(c.gnss.toInt()),
        c.recapture.toInt(),
    )

    fun checks(c: Checks): NChecks = NChecks(
        NSecurityLevel.valueOf(c.keySecurity.name),
        NBootState.valueOf(c.boot.name),
        c.deviceLocked,
        c.mockLocation,
        c.gnss.bits.toUByte(),
        c.recapture.toUByte(),
    )

    fun edit(e: NEdit): Edit = when (e) {
        is NEdit.Resize -> Edit.Resize(e.w.toInt(), e.h.toInt(), e.filter.toInt())
        is NEdit.Recompress -> Edit.Recompress(ImageFormat.valueOf(e.format.name), e.quality.toInt())
        is NEdit.Orient -> Edit.Orient(e.orientation.toInt())
        is NEdit.Crop -> Edit.Crop(rect(e.v1))
        is NEdit.Rotate -> Edit.Rotate(e.degE2.toInt())
        is NEdit.Tone -> Edit.Tone(e.exposureE2.toInt(), e.contrastE2.toInt(), e.saturationE2.toInt())
        is NEdit.Filter -> Edit.Filter(e.id.toInt(), e.strengthE2.toInt())
        is NEdit.Blur -> Edit.Blur(rect(e.rect), BlurClass.valueOf(e.`class`.name), e.sigmaE2.toInt())
    }

    fun edit(e: Edit): NEdit = when (e) {
        is Edit.Resize -> NEdit.Resize(e.w.toUInt(), e.h.toUInt(), e.filter.toUByte())
        is Edit.Recompress -> NEdit.Recompress(NImageFormat.valueOf(e.format.name), e.quality.toUByte())
        is Edit.Orient -> NEdit.Orient(e.orientation.toUByte())
        is Edit.Crop -> NEdit.Crop(rect(e.rect))
        is Edit.Rotate -> NEdit.Rotate(e.degE2.toShort())
        is Edit.Tone -> NEdit.Tone(e.exposureE2.toShort(), e.contrastE2.toShort(), e.saturationE2.toShort())
        is Edit.Filter -> NEdit.Filter(e.id.toUShort(), e.strengthE2.toUShort())
        is Edit.Blur -> NEdit.Blur(rect(e.rect), NBlurClass.valueOf(e.blurClass.name), e.sigmaE2.toUShort())
    }

    fun rect(r: NRect): Rect = Rect(r.x.toInt(), r.y.toInt(), r.w.toInt(), r.h.toInt())

    fun rect(r: Rect): NRect = NRect(r.x.toUInt(), r.y.toUInt(), r.w.toUInt(), r.h.toUInt())

    fun submission(recordId: String, view: ClaimView, s: NSubmission): Submission = Submission(
        recordId = RecordId.parse(recordId),
        keyRef = KeyRef(KeyId.fromBytes(view.keyId), view.counter.toLong()),
        isOriginal = view.kind is Kind.Original,
        wallMs = view.time.wallMs.toLong(),
        claimCbor = ByteString.wrap(s.claim),
        c2paClaimCbor = ByteString.wrap(s.c2paClaim),
        coseSign1 = ByteString.wrap(s.cose),
        fingerprints = s.fingerprints?.let(ByteString::wrap),
    )

    fun submission(s: Submission): NSubmission =
        NSubmission(s.claimCbor.toByteArray(), s.c2paClaimCbor.toByteArray(), s.coseSign1.toByteArray(), s.fingerprints?.toByteArray())

    fun anchor(a: Anchored): Anchor = Anchor(a.recordIx.toLong(), nsToMs(a.anchoredNs))

    fun submitResult(r: NSubmitResult): SubmitResult = when (r) {
        is NSubmitResult.Ok -> SubmitResult.Anchored(anchor(r.v1))
        is NSubmitResult.Dup -> SubmitResult.Duplicate(anchor(r.v1))
        is NSubmitResult.Err -> when (val e = r.v1) {
            is SubmitErr.RetryAfterNs -> SubmitResult.RetryAfter(nsToMs(e.v1))
            SubmitErr.KeyExpired -> SubmitResult.KeyExpired
            is SubmitErr.Malformed -> SubmitResult.Rejected("malformed: ${e.v1}")
            else -> SubmitResult.Rejected(submitErrCode(e))
        }
    }

    fun submitErrCode(e: SubmitErr): String = when (e) {
        is SubmitErr.Malformed -> "malformed"
        SubmitErr.TooLarge -> "too_large"
        SubmitErr.BadSignature -> "bad_signature"
        SubmitErr.UnknownKey -> "unknown_key"
        SubmitErr.KeyDistrusted -> "key_distrusted"
        SubmitErr.KeyExpired -> "key_expired"
        SubmitErr.CounterReused -> "counter_reused"
        SubmitErr.ParentUnknown -> "parent_unknown"
        SubmitErr.FingerprintMismatch -> "fingerprint_mismatch"
        is SubmitErr.RetryAfterNs -> "retry_after"
    }

    fun record(r: CertifiedRecord, decode: (ByteArray) -> ClaimView): AnchoredRecord =
        AnchoredRecord(r.recordIx.toLong(), RecordId.parse(r.recordId), claim(decode(r.claim)), nsToMs(r.anchoredNs))

    fun keyInfo(k: NKeyInfo): KeyInfo {
        val enrolled = nsToMs(k.enrolledNs)
        val changes = k.history.map { StatusChange(keyStatus(it.status), nsToMs(it.sinceNs)) }
        return KeyInfo(
            keyId = KeyId.fromBytes(k.keyId),
            security = security(k.security),
            provisioning = provisioning(k.provisioning),
            freshness = if (k.freshness == NFresh.LATE) Freshness.LATE else Freshness.FRESH,
            enrolledAtMs = enrolled,
            activeUntilMs = nsToMs(k.activeUntilNs),
            history = KeyHistory.of(changes) ?: KeyHistory.enrolled(enrolled),
        )
    }

    fun security(s: NSecurity): SecurityLevel = when (s) {
        NSecurity.STRONG_BOX -> SecurityLevel.STRONG_BOX
        NSecurity.TEE -> SecurityLevel.TEE
        NSecurity.DEV -> SecurityLevel.DEV
    }

    fun provisioning(p: NProv): Provisioning = when (p) {
        NProv.RKP -> Provisioning.RKP
        NProv.FACTORY -> Provisioning.FACTORY
        NProv.DEV -> Provisioning.DEV
    }

    fun keyStatus(s: NKeyStatus): KeyStatus = KeyStatus.valueOf(s.name)

    fun shards(s: Shards): ShardMap =
        ShardMap(s.index.map { ShardInfo(it.shard.toInt(), 0) }, s.media.map { ShardInfo(it.shard.toInt(), 0) })

    fun verdict(v: VerdictView): CheckResult = CheckResult(
        verdict = Verdict.valueOf(v.outcome.name),
        flags = EnumBits.of(v.flags.map { VerdictFlag.valueOf(it.name) }),
        reasons = EnumBits.of(v.reasons.map { VerdictReason.valueOf(it.name) }),
        declared = v.declared.map(::edit),
        regions = v.regions.map(::region),
        reference = v.reference?.let { r ->
            Reference(
                recordId = RecordId.parse(r.recordId),
                claim = claim(r.claim),
                anchor = r.anchor?.let { Anchor(it.recordIx.toLong(), nsToMs(it.anchoredNs)) },
                evidence = evidence(r.evidence),
                confidenceQ16 = r.confidenceQ16.toInt(),
                comparedQ16 = r.comparedQ16.toInt(),
                changedQ16 = r.changedQ16.toInt(),
            )
        },
        version = v.version.toInt(),
        proofsRejected = v.proofsRejected,
    )

    fun region(r: NRegion): Region = Region(r.x.toInt(), r.y.toInt(), r.w.toInt(), r.h.toInt())

    fun evidence(e: NEvidence): MatchEvidence = MatchEvidence.valueOf(e.name)
}

object Hex {
    private const val DIGITS = "0123456789abcdef"

    fun of(bytes: ByteArray): String {
        val out = CharArray(bytes.size * 2)
        for (i in bytes.indices) {
            val v = bytes[i].toInt() and 0xff
            out[2 * i] = DIGITS[v ushr 4]
            out[2 * i + 1] = DIGITS[v and 0xf]
        }
        return String(out)
    }

    fun parse(text: String): ByteArray {
        require(text.length % 2 == 0) { "hex length must be even" }
        return ByteArray(text.length / 2) { i -> ((digit(text[2 * i]) shl 4) or digit(text[2 * i + 1])).toByte() }
    }

    private fun digit(c: Char): Int = (if (c.code < 128) Character.digit(c, 16) else -1).also { require(it >= 0) { "not hex" } }
}
