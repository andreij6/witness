import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WitnessConventionsTest {
    @Test
    fun namespaceFollowsModulePath() {
        assertEquals("app.witness.core.model", witnessNamespace(":core:model"))
        assertEquals("app.witness.feature.home", witnessNamespace(":feature:home"))
    }

    @Test
    fun namespaceAvoidsJavaKeywords() {
        assertEquals("app.witness.feature.cases", witnessNamespace(":feature:case"))
        assertEquals("app.witness.nativelib.witness", witnessNamespace(":native:witness"))
    }

    @Test
    fun featuresMayOnlyReachSharedCore() {
        val allowed = allowedProjectDependencies(":feature:check")!!
        assertEquals(setOf(":core:model", ":core:designsystem", ":core:ui", ":core:domain"), allowed)
        assertTrue(":feature:proof" !in allowed)
        assertTrue(":core:data" !in allowed)
        assertTrue(":core:platform" !in allowed)
    }

    @Test
    fun domainOnlyReachesModelAndModelReachesNothing() {
        assertEquals(setOf(":core:model"), allowedProjectDependencies(":core:domain"))
        assertEquals(emptySet(), allowedProjectDependencies(":core:model"))
    }

    @Test
    fun coreAdaptersStayOffTheNativeLibrary() {
        for (m in listOf(":core:icp", ":core:verify", ":core:platform")) {
            val allowed = allowedProjectDependencies(m)!!
            assertTrue(":native:witness" !in allowed, m)
            assertTrue(allowed.none { it.startsWith(":feature:") || it == ":androidApp" }, m)
        }
        assertEquals(setOf(":core:model", ":core:domain", ":core:icp"), allowedProjectDependencies(":core:verify"))
    }

    @Test
    fun otherModulesAreUnconstrained() {
        assertNull(allowedProjectDependencies(":core:data"))
        assertNull(allowedProjectDependencies(":androidApp"))
        assertNull(allowedProjectDependencies(":featurex"))
    }
}
