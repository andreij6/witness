package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentReference
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiComponentState
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.runtime.StaticA2uiProperty
import androidx.a2ui.compose.runtime.observeA2uiComponentState
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.ui.ChoiceOption

@Composable
internal fun A2uiComponentScope.Child(reference: A2uiComponentReference, modifier: Modifier = Modifier) {
    val state = observeA2uiComponentState(reference)
    if (state is A2uiComponentState.Success) A2uiComponent(state.component, modifier)
}

internal fun A2uiComponentScope.dispatcher(action: Map<String, Any?>?): (() -> Unit)? =
    action?.let { payload -> { dispatchAction(payload) } }

internal object OptionProperties {
    val value: StaticA2uiProperty<String> = A2uiProperty.string("value", required = true)
    val label: StaticA2uiProperty<String> = A2uiProperty.string("label", required = true)

    fun list(description: String) = A2uiProperty.nestedList(
        key = "options",
        properties = listOf(value, label),
        required = true,
        description = description,
        minItems = 1,
        maxItems = MAX_OPTIONS,
    )

    fun read(items: List<A2uiComponentProperties>?): List<ChoiceOption> =
        items.orEmpty().mapNotNull { item ->
            val value = item[value] ?: return@mapNotNull null
            ChoiceOption(value, item[label] ?: value)
        }

    private const val MAX_OPTIONS = 12
}
