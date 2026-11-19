package app.witness.web

import kotlin.test.Test
import kotlin.test.assertEquals

class ResourcePathsTest {
    @Test
    fun relativeWhenNoBase() {
        assertEquals("./composeResources/a.ttf", ResourcePaths.resolve("", "composeResources/a.ttf"))
        assertEquals("./composeResources/a.ttf", ResourcePaths.resolve("", "/composeResources/a.ttf"))
    }

    @Test
    fun joinsWithExactlyOneSlash() {
        assertEquals("https://app.witness.app/x/y", ResourcePaths.resolve("https://app.witness.app/", "x/y"))
        assertEquals("https://app.witness.app/x/y", ResourcePaths.resolve("https://app.witness.app", "/x/y"))
        assertEquals("https://app.witness.app/x/y", ResourcePaths.resolve("https://app.witness.app/", "//x/y"))
    }
}
