package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.ChoiceRowBlock

internal object ChoiceRowComponent : A2uiComponent {
    private val label = A2uiProperty.string("label", required = true)
    private val value = A2uiProperty.dynamicString("value", description = "Selected option value, bound to a choice slot.")
    private val options = OptionProperties.list("Options with display labels, shown as equal pills.")

    override val name = WitnessCatalog.CHOICE_ROW
    override val description = "Single choice shown as a row of pills."
    override val properties = listOf(label, value, options)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val choices = remember(properties) { OptionProperties.read(properties[options]) }
        ChoiceRowBlock(
            label = properties[label].orEmpty(),
            options = choices,
            selected = properties.bind(value),
            onSelect = properties.bindUpdater(value),
            modifier = modifier,
        )
    }
}
