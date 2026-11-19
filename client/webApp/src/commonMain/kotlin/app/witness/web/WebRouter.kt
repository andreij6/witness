package app.witness.web

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

interface History {
    val pathname: String
    val hash: String

    fun push(path: String)

    fun replace(path: String)
}

interface KeyStore {
    fun get(name: String): String?

    fun put(name: String, value: String)
}

class WebRouter(private val history: History, private val keys: KeyStore, shell: ShellSummary? = null) {
    var route: WebRoute by mutableStateOf(initial(shell))
        private set

    var fragment: String = ""
        private set

    init {
        adoptFragment()
    }

    fun navigate(to: WebRoute) {
        if (to == route) return
        route = to
        fragment = if (to is WebRoute.Case) keys.get(keyName(to.caseId)).orEmpty() else ""
        history.push(to.path)
    }

    fun openCase(caseId: String, fragment: String) {
        if (fragment.isNotEmpty()) keys.put(keyName(caseId), fragment.removePrefix("#"))
        navigate(WebRoute.Case(caseId))
    }

    fun onPop() {
        route = WebRoute.parse(history.pathname)
        adoptFragment()
    }

    private fun initial(shell: ShellSummary?): WebRoute {
        val parsed = WebRoute.parse(history.pathname)
        val id = shell?.recordId ?: return parsed
        return if (parsed is WebRoute.Proof || parsed == WebRoute.Unknown) WebRoute.Proof(id) else parsed
    }

    private fun adoptFragment() {
        val current = route as? WebRoute.Case ?: run {
            fragment = ""
            return
        }
        val name = keyName(current.caseId)
        val hash = history.hash.removePrefix("#")
        fragment = if (hash.isNotEmpty()) {
            keys.put(name, hash)
            history.replace(current.path)
            hash
        } else {
            keys.get(name).orEmpty()
        }
    }

    companion object {
        fun keyName(caseId: String): String = "witness.case.$caseId"
    }
}
