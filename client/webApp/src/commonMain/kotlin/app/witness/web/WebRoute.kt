package app.witness.web

sealed interface WebRoute {
    val path: String

    data class Proof(val recordId: String) : WebRoute {
        override val path: String get() = PROOF + recordId
    }

    data class Case(val caseId: String) : WebRoute {
        override val path: String get() = CASE + caseId
    }

    data class Report(val page: String) : WebRoute {
        override val path: String get() = "/$page$REPORT"
    }

    data object Check : WebRoute {
        override val path: String get() = CHECK
    }

    data object Unknown : WebRoute {
        override val path: String get() = CHECK
    }

    companion object {
        const val PROOF: String = "/p/"
        const val CASE: String = "/v/"
        const val CHECK: String = "/check"
        const val REPORT: String = "/report"
        const val MAX_SEGMENT: Int = 64

        fun parse(pathname: String): WebRoute {
            val path = pathname.trimEnd('/')
            if (path == CHECK) return Check
            if (path.endsWith(REPORT) && path.length > REPORT.length + 1) {
                val page = path.substring(1, path.length - REPORT.length)
                return when (parse("/$page")) {
                    is Proof, is Case -> Report(page)
                    else -> Unknown
                }
            }
            return when {
                path.startsWith(PROOF) -> path.segmentAfter(PROOF)?.let(::Proof) ?: Unknown
                path.startsWith(CASE) -> path.segmentAfter(CASE)?.let(::Case) ?: Unknown
                else -> Unknown
            }
        }

        private fun String.segmentAfter(prefix: String): String? {
            val n = length - prefix.length
            if (n !in 1..MAX_SEGMENT || indexOf('/', prefix.length) >= 0) return null
            for (i in prefix.length until length) if (!this[i].isSegmentChar()) return null
            return substring(prefix.length)
        }

        private fun Char.isSegmentChar(): Boolean = this in 'A'..'Z' || this in 'a'..'z' || this in '0'..'9' || this == '-'
    }
}
