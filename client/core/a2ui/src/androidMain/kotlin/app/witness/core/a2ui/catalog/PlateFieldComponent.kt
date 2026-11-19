package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.SlotBinder
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.PlateFieldBlock

internal object PlateFieldComponent : A2uiComponent {
    private val label = A2uiProperty.string("label", required = true)
    private val value = A2uiProperty.dynamicString("value", description = "License plate, bound to a plate slot.")
    private val caption = A2uiProperty.dynamicString("caption", description = "Provenance, e.g. Read from photo 3.")

    override val name = WitnessCatalog.PLATE_FIELD
    override val description = "Editable license plate in monospace with its provenance chip."
    override val properties = listOf(label, value, caption)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val update = properties.bindUpdater(value)
        PlateFieldBlock(
            label = properties[label].orEmpty(),
            value = properties.bind(value),
            caption = properties.bind(caption),
            onValueChange = update?.let { write -> { raw -> write(SlotBinder.sanitizePlateInput(raw)) } },
            modifier = modifier,
        )
    }
}
