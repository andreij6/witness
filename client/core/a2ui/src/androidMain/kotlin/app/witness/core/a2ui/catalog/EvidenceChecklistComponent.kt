package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.ChecklistItem
import app.witness.core.a2ui.ui.EvidenceChecklistCard

internal object EvidenceChecklistComponent : A2uiComponent {
    private val title = A2uiProperty.string("title", required = true, description = "Header, e.g. Still recommended.")
    private val key = A2uiProperty.string("key", required = true)
    private val label = A2uiProperty.string("label", required = true)
    private val done = A2uiProperty.dynamicBoolean("done", description = "True once the evidence is captured.")
    private val meta = A2uiProperty.dynamicString("meta", description = "Which photos cover it, e.g. Photo 2.")
    private val action = A2uiProperty.action("action", description = "capture_for event for this item.")
    private val items = A2uiProperty.nestedList(
        key = "items",
        properties = listOf(key, label, done, meta, action),
        required = true,
        minItems = 1,
        maxItems = 12,
    )
    private val flagAction = A2uiProperty.action("flagAction")

    override val name = WitnessCatalog.EVIDENCE_CHECKLIST
    override val description = "Evidence coach: what is captured and what to capture next."
    override val properties = listOf(title, items, flagAction)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val entries = properties[items].orEmpty()
        val rows = ArrayList<ChecklistItem>(entries.size)
        val actions = HashMap<String, Map<String, Any?>>(entries.size * 2)
        for (entry in entries) {
            val itemKey = entry[key] ?: continue
            rows += ChecklistItem(itemKey, entry[label] ?: itemKey, entry.bind(done) == true, entry.bind(meta))
            entry[action]?.let { actions[itemKey] = it }
        }
        EvidenceChecklistCard(
            title = properties[title].orEmpty(),
            items = rows,
            onCapture = { itemKey -> actions[itemKey]?.let(::dispatchAction) },
            onFlag = dispatcher(properties[flagAction]),
            modifier = modifier,
        )
    }
}
