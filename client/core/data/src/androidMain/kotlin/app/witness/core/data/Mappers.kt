package app.witness.core.data

import app.witness.core.data.codec.EditCodec
import app.witness.core.data.codec.ShareCodec
import app.witness.core.model.Anchor
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureId
import app.witness.core.model.Case
import app.witness.core.model.Dims
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocalCaseId
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId

internal fun CaptureRow.toCapture(): Capture {
    val c = capture
    val keyRef = if (c.keyId != null && c.counter != null) KeyRef(KeyId.parse(c.keyId), c.counter) else null
    return Capture(
        recordId = RecordId.parse(c.recordId),
        captureId = c.captureId?.let(CaptureId::fromLong),
        origin = c.origin,
        status = c.status,
        createdAtMs = c.createdAtMs,
        keyRef = keyRef,
        locationConfidence = c.locationConfidence,
        dims = Dims(c.width, c.height),
        caseId = caseId?.let(::LocalCaseId),
        parent = parent?.let(RecordId::parse),
        edits = edits?.takeIf { it.isNotEmpty() }?.let(EditCodec::decode).orEmpty(),
        anchor = if (c.anchorIx != null && c.anchoredAtMs != null) Anchor(c.anchorIx, c.anchoredAtMs) else null,
        label = c.label,
        place = c.place,
        caption = c.caption,
        altText = c.altText,
        publicErased = c.publicErased,
    )
}

internal fun Capture.toRow(): CaptureRow = CaptureRow(
    CaptureEntity(
        recordId = recordId.text,
        captureId = captureId?.value,
        origin = origin,
        status = status,
        createdAtMs = createdAtMs,
        keyId = keyRef?.keyId?.text,
        counter = keyRef?.counter,
        locationConfidence = locationConfidence,
        width = dims.width,
        height = dims.height,
        anchorIx = anchor?.recordIx,
        anchoredAtMs = anchor?.anchoredAtMs,
        label = label,
        place = place,
        caption = caption,
        altText = altText,
        publicErased = publicErased,
    ),
    caseId = caseId?.value,
    parent = parent?.text,
    edits = if (parent == null) null else EditCodec.encode(edits),
)

internal fun OutboxEntity.toEntry() = OutboxEntry(
    Submission(
        RecordId.parse(recordId),
        KeyRef(KeyId.parse(keyId), counter),
        isOriginal,
        wallMs,
        ByteString.wrap(claimCbor),
        ByteString.wrap(c2paClaimCbor),
        ByteString.wrap(coseSign1),
        fingerprints?.let(ByteString::wrap),
    ),
    attempts,
    notBeforeMs,
    state,
    lastError,
)

internal fun OutboxEntry.toEntity(): OutboxEntity {
    val s = submission
    return OutboxEntity(
        s.recordId.text, s.keyRef.keyId.text, s.keyRef.counter, s.isOriginal, s.wallMs,
        s.claimCbor.toByteArray(), s.c2paClaimCbor.toByteArray(), s.coseSign1.toByteArray(),
        attempts, notBeforeMs, state, lastError, s.fingerprints?.toByteArray(),
    )
}

internal fun CaseEntity.toCase() =
    Case(LocalCaseId(id), title, TemplateId(templateId), createdAtMs, updatedAtMs, summary, ShareCodec.decode(shares), SummarySource.parse(summarySource))

internal fun Case.toEntity() =
    CaseEntity(id.value, title, templateId.value, createdAtMs, updatedAtMs, summary, ShareCodec.encode(shares), summarySource?.name)

internal inline fun <A, B> List<A>.mapFast(transform: (A) -> B): List<B> {
    val out = ArrayList<B>(size)
    for (i in indices) out.add(transform(this[i]))
    return out
}
