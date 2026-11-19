package app.witness.core.a2ui

import app.witness.core.a2ui.resources.Res
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateSchema
import org.jetbrains.compose.resources.ExperimentalResourceApi

class TemplateStore(templates: List<Template>) : TemplateCatalog {
    val all: List<Template> = templates.toList()

    private val byId: Map<String, Template> = HashMap<String, Template>(all.size * 2).apply {
        for (template in all) require(put(template.id, template) == null) { "Duplicate template '${template.id}'" }
    }

    private val schemas: List<TemplateSchema> = all.map(Template::schema)

    operator fun get(id: String): Template? = byId[id]

    override suspend fun templates(): List<TemplateSchema> = schemas

    fun bind(suggestion: Suggestion, fallback: Suggestion? = null): Binding? =
        byId[suggestion.templateId.value]?.let { SlotBinder.bind(it, suggestion, fallback) }

    companion object {
        val bundledIds: List<String> = listOf("car_accident", "property_damage", "rental_condition", "general")

        fun decode(sources: List<String>): TemplateStore = TemplateStore(sources.map(TemplateCodec::decode))

        fun bundledPath(id: String): String = "files/templates/$id.json"

        @OptIn(ExperimentalResourceApi::class)
        suspend fun bundled(read: suspend (String) -> ByteArray = { Res.readBytes(it) }): TemplateStore =
            decode(bundledIds.map { read(bundledPath(it)).decodeToString() })
    }
}
