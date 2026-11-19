package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.domain.port.TemplateCatalog
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame

class TemplateStoreTest {
    private val car = TestTemplates.template("car_accident")
    private val rental = TestTemplates.template("rental_condition")
    private val store = TemplateStore(listOf(car, rental))

    @Test
    fun looksUpById() {
        assertSame(car, store["car_accident"])
        assertSame(rental, store["rental_condition"])
        assertNull(store["property_damage"])
    }

    @Test
    fun keepsOrder() = assertEquals(listOf("car_accident", "rental_condition"), store.all.map { it.id })

    @Test
    fun rejectsDuplicateIds() {
        assertFailsWith<IllegalArgumentException> { TemplateStore(listOf(car, TestTemplates.template("car_accident"))) }
    }

    @Test
    fun emptyStoreIsValid() {
        val empty = TemplateStore(emptyList())
        assertEquals(0, empty.all.size)
        assertNull(empty.bind(Suggestion(TemplateId("car_accident"), emptyList())))
    }

    @Test
    fun bindRoutesBySuggestionTemplate() {
        val binding = store.bind(Suggestion(TemplateId("rental_condition"), listOf(Slot("summary", text = "ok"))))
        assertNotNull(binding)
        assertSame(rental, binding.template)
        assertEquals(1, binding.values.size)
    }

    @Test
    fun bindUnknownTemplateIsNull() = assertNull(store.bind(Suggestion(TemplateId("workplace_incident"), emptyList())))

    @Test
    fun decodesSources() {
        val decoded = TemplateStore.decode(listOf(TestTemplates.json("a"), TestTemplates.json("b")))
        assertEquals(listOf("a", "b"), decoded.all.map { it.id })
    }

    @Test
    fun decodeFailsOnAnyBadSource() {
        assertFailsWith<TemplateException> { TemplateStore.decode(listOf(TestTemplates.json("a"), "{}")) }
    }

    @Test
    fun bundledIdsMatchDesign() =
        assertEquals(listOf("car_accident", "property_damage", "rental_condition", "general"), TemplateStore.bundledIds)

    @Test
    fun bundledPathPointsAtTemplateResources() =
        assertEquals("files/templates/car_accident.json", TemplateStore.bundledPath("car_accident"))

    @Test
    fun bundledReadsEveryIdThroughTheReader() = runTest {
        val requested = ArrayList<String>()
        val loaded = TemplateStore.bundled { path ->
            requested += path
            TestTemplates.json(path.substringAfterLast('/').removeSuffix(".json")).encodeToByteArray()
        }
        assertEquals(TemplateStore.bundledIds.map(TemplateStore::bundledPath), requested)
        assertEquals(TemplateStore.bundledIds, loaded.all.map { it.id })
    }

    @Test
    fun servesTheDomainTemplateCatalogPort() = runTest {
        val catalog: TemplateCatalog = store
        val schemas = catalog.templates()
        assertEquals(listOf(TemplateId("car_accident"), TemplateId("rental_condition")), schemas.map { it.id })
        assertSame(schemas, catalog.templates())
        assertEquals(car.slots.map { it.id }, schemas[0].slots.map { it.id })
    }
}
