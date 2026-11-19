package app.witness.core.icp.id

import kotlin.jvm.JvmInline

@JvmInline
value class RecordId private constructor(val text: String) {
    val bytes: ByteArray get() = CrockfordBase32.decode(text, LEN)

    val grouped: String
        get() = buildString(GROUPED_LEN) {
            for (i in 0 until TEXT_LEN step GROUP) {
                if (i > 0) append('-')
                append(text, i, i + GROUP)
            }
        }

    override fun toString(): String = text

    companion object {
        const val LEN: Int = 10
        const val TEXT_LEN: Int = 16
        const val GROUP: Int = 4
        const val GROUPED_LEN: Int = 19

        fun of(bytes: ByteArray): RecordId {
            require(bytes.size == LEN) { "record id needs $LEN bytes, got ${bytes.size}" }
            return RecordId(CrockfordBase32.encode(bytes))
        }

        fun parseOrThrow(text: CharSequence): RecordId = of(CrockfordBase32.decode(text, LEN))

        fun parse(text: CharSequence): RecordId? {
            val out = ByteArray(LEN)
            return if (CrockfordBase32.decodeInto(text, out) == null) of(out) else null
        }
    }
}
