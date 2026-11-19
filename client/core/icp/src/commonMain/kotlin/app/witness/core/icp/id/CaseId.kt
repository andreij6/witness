package app.witness.core.icp.id

import kotlin.jvm.JvmInline

@JvmInline
value class CaseId private constructor(val text: String) {
    val bytes: ByteArray get() = CrockfordBase32.decode(text, LEN)

    val shard: Int
        get() = (CrockfordBase32.symbolValue(text[0]) shl 3) or (CrockfordBase32.symbolValue(text[1]) ushr 2)

    override fun toString(): String = text

    companion object {
        const val LEN: Int = 16
        const val RANDOM_LEN: Int = 15
        const val TEXT_LEN: Int = 26

        fun of(bytes: ByteArray): CaseId {
            require(bytes.size == LEN) { "case id needs $LEN bytes, got ${bytes.size}" }
            return CaseId(CrockfordBase32.encode(bytes))
        }

        fun of(shard: Int, random: ByteArray): CaseId {
            require(shard in 0..255) { "shard $shard out of range" }
            require(random.size == RANDOM_LEN) { "case id needs $RANDOM_LEN random bytes, got ${random.size}" }
            val bytes = ByteArray(LEN)
            bytes[0] = shard.toByte()
            random.copyInto(bytes, 1)
            return of(bytes)
        }

        fun parseOrThrow(text: CharSequence): CaseId = of(CrockfordBase32.decode(text, LEN))

        fun parse(text: CharSequence): CaseId? {
            val out = ByteArray(LEN)
            return if (CrockfordBase32.decodeInto(text, out) == null) of(out) else null
        }
    }
}
