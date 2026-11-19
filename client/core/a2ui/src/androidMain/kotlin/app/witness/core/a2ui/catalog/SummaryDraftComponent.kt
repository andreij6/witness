package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.WitnessEventSink
import app.witness.core.a2ui.ui.LocalSummaryHeader
import app.witness.core.a2ui.ui.SummaryDraftCard

internal object SummaryDraftComponent : A2uiComponent {
    private val title = A2uiProperty.string("title", required = true)
    private val text = A2uiProperty.dynamicString("text", description = "Draft text, bound to a text slot.")
    private val editAction = A2uiProperty.action("editAction")
    private val flagAction = A2uiProperty.action("flagAction")

    override val name = WitnessCatalog.SUMMARY_DRAFT
    override val description = "Generated text block the user can edit or flag."
    override val properties = listOf(title, text, editAction, flagAction)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val edit = properties[editAction]
        SummaryDraftCard(
            title = properties[title].orEmpty(),
            text = properties.bind(text),
            onEdit = dispatcher(edit),
            onFlag = dispatcher(properties[flagAction]),
            modifier = modifier,
            header = LocalSummaryHeader.current.forTarget(WitnessEventSink.targetOf(edit)),
        )
    }
}
