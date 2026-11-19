package app.witness.web

object ResourcePaths {
    fun resolve(base: String, path: String): String {
        val clean = path.trimStart('/')
        if (base.isEmpty()) return "./$clean"
        return if (base.endsWith('/')) base + clean else "$base/$clean"
    }
}
