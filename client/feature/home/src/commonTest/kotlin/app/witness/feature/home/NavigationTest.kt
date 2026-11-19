package app.witness.feature.home

import kotlinx.serialization.json.Json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class NavigationTest {
    @Test
    fun everyKeyRoundTripsThroughSerialization() {
        for (key in HomeKey.all) assertEquals(key, Json.decodeFromString(HomeKey.serializer(), Json.encodeToString(HomeKey.serializer(), key)))
    }

    @Test
    fun keysAreDistinct() = assertEquals(HomeKey.all.size, HomeKey.all.toSet().size)

    @Test
    fun onboardingKeysExcludeMainScreens() {
        assertTrue(HomeKey.all.containsAll(HomeKey.onboarding))
        assertTrue(HomeKey.Home !in HomeKey.onboarding)
        assertTrue(HomeKey.Search !in HomeKey.onboarding)
    }

    @Test
    fun finishOnboardingResetsToHomeThenOpensCamera() {
        val log = ArrayList<String>()
        val nav = navigator(log)
        nav.finishOnboarding(openCamera = true)
        assertEquals(listOf("reset:Home", "camera"), log)
    }

    @Test
    fun finishOnboardingWithoutCamera() {
        val log = ArrayList<String>()
        navigator(log).finishOnboarding(openCamera = false)
        assertEquals(listOf("reset:Home"), log)
    }

    private fun navigator(log: MutableList<String>) = HomeNavigator(
        push = { log += "push:$it" },
        resetTo = { log += "reset:$it" },
        back = { log += "back" },
        openCamera = { log += "camera" },
        openCheck = { log += "check" },
        openSettings = { log += "settings" },
        openItem = { log += "item:$it" },
        openCase = { log += "case:${it.value}" },
        newCase = { log += "new-case" },
    )
}
