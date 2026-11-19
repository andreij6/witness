package app.witness.core.icp.witness

import app.witness.core.icp.witness.WireJson.array
import app.witness.core.icp.witness.WireJson.asObject
import app.witness.core.icp.witness.WireJson.int
import app.witness.core.icp.witness.WireJson.str
import app.witness.core.icp.witness.WireJson.strOrNull
import app.witness.core.icp.witness.WireJson.u64OrNull
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.RecordId

class CaseEntry(val recordId: RecordId?, val jpeg: ByteString, val capturedAtMs: Long?)

sealed interface CaseOpening {
    class Opened(val title: String, val summary: String?, val items: List<CaseEntry>, val expiresAtMs: Long? = null) : CaseOpening

    data object NotFound : CaseOpening

    data object Tampered : CaseOpening
}

class BridgeCases(private val bridge: WitnessBridge) {
    suspend fun open(caseId: CaseId, fragment: String): CaseOpening {
        val opened = bridge.openCase(caseId.text, fragment)
        return when (opened.status) {
            OpenedBundle.OPENED -> decode(opened) ?: CaseOpening.Tampered
            OpenedBundle.NOT_FOUND -> CaseOpening.NotFound
            else -> CaseOpening.Tampered
        }
    }

    private fun decode(opened: OpenedBundle): CaseOpening.Opened? {
        val bundle = opened.bundle ?: return null
        val o = WireJson.parse(opened.json ?: return null)
        val items = o.array("items").map { element ->
            val item = element.asObject()
            val offset = item.int("offset")
            val len = item.int("len")
            if (offset < 0 || len <= 0 || offset > bundle.size - len) return null
            CaseEntry(RecordId.parseOrNull(item.str("recordId")), ByteString.wrap(bundle.copyOfRange(offset, offset + len)), item.u64OrNull("wallMs"))
        }
        if (items.isEmpty()) return null
        return CaseOpening.Opened(o.str("title"), o.strOrNull("summary"), items, o.u64OrNull("expiresMs"))
    }
}
