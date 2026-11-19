package app.witness.feature.home

import androidx.navigation3.runtime.NavKey
import kotlinx.serialization.Serializable

@Serializable
sealed interface HomeKey : NavKey {
    @Serializable
    data object Splash : HomeKey

    @Serializable
    data object Welcome : HomeKey

    @Serializable
    data object Permissions : HomeKey

    @Serializable
    data object DeviceSetup : HomeKey

    @Serializable
    data object Unsupported : HomeKey

    @Serializable
    data object Home : HomeKey

    @Serializable
    data object Anchoring : HomeKey

    @Serializable
    data object Search : HomeKey

    companion object {
        val all: List<HomeKey> = listOf(Splash, Welcome, Permissions, DeviceSetup, Unsupported, Home, Anchoring, Search)
        val onboarding: Set<HomeKey> = setOf(Splash, Welcome, Permissions, DeviceSetup, Unsupported)
    }
}
