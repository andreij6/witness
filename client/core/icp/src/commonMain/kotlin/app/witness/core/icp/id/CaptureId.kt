package app.witness.core.icp.id

import kotlin.jvm.JvmInline

@JvmInline
value class CaptureId private constructor(val value: Long) {
    val schema: Int get() = (value ushr RANDOM_BITS).toInt()
    val random: Long get() = value and RANDOM_MASK
    val high: Int get() = (value ushr 32).toInt()
    val low: Int get() = value.toInt()

    companion object {
        const val BITS: Int = 61
        const val RANDOM_BITS: Int = 56
        const val SCHEMA_BITS: Int = 5
        const val RANDOM_MASK: Long = (1L shl RANDOM_BITS) - 1
        const val MAX: Long = (1L shl BITS) - 1

        fun of(random: Long, schema: Int): CaptureId? =
            if (random !in 0..RANDOM_MASK || schema !in 0 until (1 shl SCHEMA_BITS)) null
            else CaptureId((schema.toLong() shl RANDOM_BITS) or random)

        fun fromLong(value: Long): CaptureId? = if (value in 0..MAX) CaptureId(value) else null
    }
}
