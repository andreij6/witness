package app.witness.web

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.window.ComposeViewport
import app.witness.core.domain.Clock
import app.witness.core.icp.witness.JsWitnessBridge
import kotlinx.browser.document
import kotlinx.browser.sessionStorage
import kotlinx.browser.window
import org.jetbrains.compose.resources.ExperimentalResourceApi
import org.jetbrains.compose.resources.configureWebResources
import org.w3c.dom.HTMLCanvasElement
import kotlin.time.ExperimentalTime

object BrowserHistory : History {
    override val pathname: String get() = window.location.pathname
    override val hash: String get() = window.location.hash

    override fun push(path: String) = window.history.pushState(null, "", path)

    override fun replace(path: String) = window.history.replaceState(null, "", path + window.location.search)
}

object SessionKeys : KeyStore {
    override fun get(name: String): String? = runCatching { sessionStorage.getItem(name) }.getOrNull()

    override fun put(name: String, value: String) {
        runCatching { sessionStorage.setItem(name, value) }
    }
}

object Mount {
    const val APP_ID: String = "app"
    const val READY_CLASS: String = "compose-ready"
    const val RESOURCES_META: String = "witness-resources"
    const val NO_WEBGL_NOTE: String = "This browser can't run the interactive checker (WebGL 2 is off). The summary on this page is still accurate."

    fun hasWebGl2(): Boolean = runCatching {
        (document.createElement("canvas") as HTMLCanvasElement).getContext("webgl2") != null
    }.getOrDefault(false)

    fun resourceBase(): String = document.querySelector("meta[name=\"$RESOURCES_META\"]")?.getAttribute("content").orEmpty()

    fun shellJson(): String? = document.getElementById(ShellSummary.ELEMENT_ID)?.textContent

    fun noteNoWebGl() {
        val note = document.createElement("p")
        note.className = "witness-note"
        note.textContent = NO_WEBGL_NOTE
        (document.querySelector("main") ?: document.body)?.appendChild(note)
    }
}

@OptIn(ExperimentalComposeUiApi::class, ExperimentalResourceApi::class, ExperimentalTime::class)
fun main() {
    if (!Mount.hasWebGl2()) {
        Mount.noteNoWebGl()
        return
    }
    val base = Mount.resourceBase()
    configureWebResources { resourcePathMapping { path -> ResourcePaths.resolve(base, path) } }
    val router = WebRouter(BrowserHistory, SessionKeys, ShellSummary.parse(Mount.shellJson()))
    window.addEventListener("popstate", { router.onPop() })
    val graph = WebGraph(Clock { kotlin.time.Clock.System.now().toEpochMilliseconds() }, JsWitnessBridge)
    val root = document.getElementById(Mount.APP_ID) ?: return
    ComposeViewport(root) { WebApp(router, graph) }
    document.body?.classList?.add(Mount.READY_CLASS)
}
