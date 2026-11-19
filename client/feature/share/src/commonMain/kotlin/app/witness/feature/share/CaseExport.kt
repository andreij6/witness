package app.witness.feature.share

import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.Links
import app.witness.core.domain.usecase.Geohash
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationPrecision
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject

enum class ExportFormat(val extension: String, val mimeType: String) {
    ZIP("zip", "application/zip"),
    PDF("pdf", "application/pdf"),
}

data class ExportRequest(val caseId: LocalCaseId, val format: ExportFormat, val exactLocation: Boolean)

sealed interface ExportResult {
    data class Saved(val name: String, val location: String, val bytes: Long, val items: Int) : ExportResult

    data object Unsupported : ExportResult

    data object NothingToExport : ExportResult
}

fun interface CaseExporter {
    suspend fun export(request: ExportRequest): ExportResult
}

interface ExportOutput : ByteSink {
    suspend fun commit(): String

    suspend fun abort()
}

fun interface ExportSink {
    suspend fun open(name: String, mimeType: String): ExportOutput
}

class ZipCaseExporter(
    private val cases: CaseStore,
    private val captures: CaptureStore,
    private val sink: ExportSink,
    private val clock: Clock,
    private val links: Links = Links(),
) : CaseExporter {
    override suspend fun export(request: ExportRequest): ExportResult {
        if (request.format != ExportFormat.ZIP) return ExportResult.Unsupported
        val case = cases.get(request.caseId) ?: return ExportResult.NothingToExport
        val items = captures.observeAll().first().filter { it.caseId == case.id && it.isVerified }.sortedBy { it.createdAtMs }
        if (items.isEmpty()) return ExportResult.NothingToExport
        val name = fileName(case.title, ExportFormat.ZIP)
        val out = sink.open(name, ExportFormat.ZIP.mimeType)
        try {
            val (time, date) = dosStamp(clock.nowEpochMillis())
            val zip = ZipWriter(out, time, date)
            var written = 0
            val entries = ArrayList<JsonObject>(items.size)
            for ((index, c) in items.withIndex()) {
                val jpeg = captures.readImage(c.recordId) ?: continue
                val photo = photoName(index, c)
                zip.add(photo, jpeg.toByteArray())
                val record = proofRecord(c, photo, captures.opening(c.recordId), request.exactLocation)
                zip.add(PROOF_DIR + c.recordId.text + ".json", record.toString().encodeToByteArray())
                entries += record
                written++
            }
            if (written == 0) {
                out.abort()
                return ExportResult.NothingToExport
            }
            zip.add(CASE_FILE, caseRecord(case, entries).toString().encodeToByteArray())
            zip.add(README_FILE, readme(case).encodeToByteArray())
            zip.finish()
            return ExportResult.Saved(name, out.commit(), zip.bytesWritten, written)
        } catch (e: Throwable) {
            withContext(NonCancellable) { out.abort() }
            throw e
        }
    }

    fun proofRecord(capture: Capture, photo: String, opening: LocationOpening?, exact: Boolean): JsonObject = buildJsonObject {
        put("record_id", capture.recordId.text)
        put("file", photo)
        put("proof_url", links.proof(capture.recordId))
        put("origin", capture.origin.name.lowercase())
        put("status", capture.status.name.lowercase())
        put("created_at_ms", capture.createdAtMs)
        capture.parent?.let { put("parent", it.text) }
        capture.keyRef?.let {
            put("key_id", it.keyId.text)
            put("counter", it.counter)
        }
        capture.anchor?.let {
            putJsonObject("anchor") {
                put("record_ix", it.recordIx)
                put("anchored_at_ms", it.anchoredAtMs)
            }
        }
        if (capture.edits.isNotEmpty()) putJsonArray("edits") { capture.edits.forEach { add(it.kind.name.lowercase()) } }
        putJsonObject("location") {
            put("confidence", capture.locationConfidence.name.lowercase())
            if (opening != null && opening.inRange) {
                if (exact) {
                    put("lat", e7(opening.latE7))
                    put("lon", e7(opening.lonE7))
                    put("accuracy_m", opening.accuracyCm / 100.0)
                } else {
                    put("geohash", Geohash.of(opening, LocationPrecision.NEIGHBORHOOD.geohashChars!!))
                }
            }
        }
    }

    private fun caseRecord(case: Case, entries: List<JsonObject>): JsonObject = buildJsonObject {
        put("title", case.title)
        put("template", case.templateId.value)
        case.summary?.let { put("summary", it) }
        case.summarySource?.takeIf { case.summary != null }?.let { put("summary_source", it.name.lowercase()) }
        put("created_at_ms", case.createdAtMs)
        put("exported_at_ms", clock.nowEpochMillis())
        putJsonArray("items") { entries.forEach { add(it) } }
    }

    private fun readme(case: Case): String =
        "Witness case export: ${case.title}\n\n" +
            "photos/ holds each sealed photo exactly as captured or as a declared edit.\n" +
            "proof/ holds one record per photo, with its link on the Witness registry.\n" +
            "Anyone can check a photo at ${links.proofBase.removeSuffix("p/")}check without the app.\n"

    companion object {
        const val PROOF_DIR: String = "proof/"
        const val PHOTO_DIR: String = "photos/"
        const val CASE_FILE: String = "case.json"
        const val README_FILE: String = "README.txt"
        private const val MAX_SLUG: Int = 40

        fun photoName(index: Int, capture: Capture): String {
            val n = (index + 1).toString().padStart(2, '0')
            return "$PHOTO_DIR$n-${capture.recordId.text}.jpg"
        }

        fun fileName(title: String, format: ExportFormat): String {
            val slug = StringBuilder(MAX_SLUG)
            var dash = false
            for (ch in title.lowercase()) {
                if (slug.length >= MAX_SLUG) break
                if (ch in 'a'..'z' || ch in '0'..'9') {
                    if (dash && slug.isNotEmpty()) slug.append('-')
                    slug.append(ch)
                    dash = false
                } else {
                    dash = true
                }
            }
            val base = if (slug.isEmpty()) "case" else slug.toString()
            return "witness-$base.${format.extension}"
        }

        fun e7(value: Int): Double = value / 1e7

        fun dosStamp(epochMs: Long): Pair<Int, Int> {
            val days = epochMs.floorDiv(ShareText.DAY_MS)
            val secondsOfDay = (epochMs.mod(ShareText.DAY_MS) / 1000).toInt()
            val (y, m, d) = ShareText.civil(days)
            return ZipWriter.dosDateTime(y, m, d, secondsOfDay / 3600, secondsOfDay / 60 % 60, secondsOfDay % 60)
        }
    }
}
