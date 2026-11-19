package app.witness.system

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import app.witness.BuildConfig
import app.witness.R
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.xmlpull.v1.XmlPullParser
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
class NetworkSecurityConfigTest {
    private fun cleartext(): Pair<Set<String>, List<String>> {
        val xml = ApplicationProvider.getApplicationContext<Application>().resources.getXml(R.xml.network_security_config)
        val permits = mutableSetOf<String>()
        val domains = mutableListOf<String>()
        var tag = ""
        while (xml.next() != XmlPullParser.END_DOCUMENT) {
            when (xml.eventType) {
                XmlPullParser.START_TAG -> {
                    tag = xml.name
                    xml.getAttributeValue(null, "cleartextTrafficPermitted")?.let { permits += "$tag=$it" }
                }
                XmlPullParser.TEXT -> if (tag == "domain") domains += xml.text.trim()
            }
        }
        return permits to domains
    }

    @Test
    fun cleartextIsAllowedOnlyToLocalHostsAndOnlyInLocalBuilds() {
        val (permits, domains) = cleartext()
        if (BuildConfig.ICP_FETCH_ROOT_KEY) {
            assertEquals(setOf("domain-config=true"), permits)
            assertEquals(listOf("10.0.2.2", "localhost", "127.0.0.1"), domains)
        } else {
            assertEquals(setOf("base-config=false"), permits)
            assertEquals(emptyList(), domains)
        }
    }

    @Test
    fun releaseSourcesForbidCleartextEverywhere() {
        val main = File("src/main/res/xml/network_security_config.xml").readText()
        assertTrue("<base-config cleartextTrafficPermitted=\"false\" />" in main)
        assertFalse("true" in main)
        assertFalse("<domain" in main)
    }
}
