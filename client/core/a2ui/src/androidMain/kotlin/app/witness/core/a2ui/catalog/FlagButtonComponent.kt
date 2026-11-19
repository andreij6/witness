package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.FlagIconButton

internal object FlagButtonComponent : A2uiComponent {
    private val action = A2uiProperty.action("action", required = true, description = "flag event naming its target.")

    override val name = WitnessCatalog.FLAG_BUTTON
    override val description = "Flag control for reporting a generated part."
    override val properties = listOf(action)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val payload = properties[action] ?: return
        FlagIconButton(onClick = { dispatchAction(payload) }, modifier = modifier)
    }
}
