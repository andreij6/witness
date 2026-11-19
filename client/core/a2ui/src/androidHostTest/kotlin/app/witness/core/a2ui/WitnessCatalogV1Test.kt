package app.witness.core.a2ui

import androidx.a2ui.compose.ui.toJsonSchemaString
import app.witness.core.a2ui.catalog.WitnessCatalogV1
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class WitnessCatalogV1Test {
    private val catalog = WitnessCatalogV1.catalog

    @Test
    fun usesWitnessCatalogId() = assertEquals(WitnessCatalog.ID, catalog.id)

    @Test
    fun registersExactlyTheDeclaredComponents() {
        assertEquals(WitnessCatalog.componentNames, catalog.components.map { it.name }.toSet())
        assertEquals(catalog.components.size, WitnessCatalog.componentNames.size)
    }

    @Test
    fun componentsAreLookupable() {
        WitnessCatalog.componentNames.forEach { assertNotNull(catalog.components[it], it) }
    }

    @Test
    fun everyComponentDescribesItself() {
        catalog.components.forEach {
            assertTrue(it.description.isNotBlank(), it.name)
            assertTrue(it.properties.isNotEmpty(), it.name)
        }
    }

    @Test
    fun propertyKeysAreUniquePerComponent() {
        catalog.components.forEach { component ->
            val keys = component.properties.map { it.key }
            assertEquals(keys.size, keys.toSet().size, component.name)
        }
    }

    @Test
    fun schemaAdvertisesEveryComponent() {
        val schema = catalog.toJsonSchemaString()
        WitnessCatalog.componentNames.forEach { assertTrue(it in schema, it) }
    }

    @Test
    fun catalogIsASingleton() = assertTrue(WitnessCatalogV1.catalog === catalog)
}
