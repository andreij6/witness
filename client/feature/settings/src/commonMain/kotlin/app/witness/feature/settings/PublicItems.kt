package app.witness.feature.settings

import app.witness.core.domain.usecase.EraseMine
import app.witness.core.model.Capture
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters

data class PublicItemUi(val recordId: RecordId, val title: String, val subtitle: String)

object PublicItems {
    const val UNTITLED = "Photo"

    fun of(all: List<Capture>): List<PublicItemUi> {
        val copies = HashMap<RecordId, Int>()
        for (c in all) {
            if (!EraseMine.isErasable(c)) continue
            val root = c.parent ?: c.recordId
            copies[root] = (copies[root] ?: 0) + 1
        }
        if (copies.isEmpty()) return emptyList()
        return all.filter { it.parent == null && it.recordId in copies }
            .sortedWith(compareByDescending<Capture> { it.createdAtMs }.thenBy { it.recordId.text })
            .map { c -> PublicItemUi(c.recordId, titleOf(c), Formatters.joinDot(c.recordId.grouped(), Formatters.plural(copies.getValue(c.recordId), "public copy", "public copies"))) }
    }

    private fun titleOf(c: Capture): String = c.label?.takeIf(String::isNotBlank) ?: c.place?.takeIf(String::isNotBlank) ?: UNTITLED
}
