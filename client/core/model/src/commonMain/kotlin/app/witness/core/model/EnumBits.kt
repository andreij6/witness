package app.witness.core.model

import kotlin.enums.enumEntries
import kotlin.jvm.JvmInline

@JvmInline
value class EnumBits<E : Enum<E>>(val bits: Int) {
    val isEmpty: Boolean get() = bits == 0
    val size: Int get() = bits.countOneBits()

    operator fun contains(flag: E): Boolean = bits and (1 shl flag.ordinal) != 0

    fun containsAll(other: EnumBits<E>): Boolean = bits and other.bits == other.bits

    fun intersects(other: EnumBits<E>): Boolean = bits and other.bits != 0

    operator fun plus(flag: E): EnumBits<E> = EnumBits(bits or (1 shl flag.ordinal))

    operator fun plus(other: EnumBits<E>): EnumBits<E> = EnumBits(bits or other.bits)

    operator fun minus(flag: E): EnumBits<E> = EnumBits(bits and (1 shl flag.ordinal).inv())

    fun with(flag: E, on: Boolean): EnumBits<E> = if (on) plus(flag) else minus(flag)

    companion object {
        fun <E : Enum<E>> none(): EnumBits<E> = EnumBits(0)

        fun <E : Enum<E>> of(vararg flags: E): EnumBits<E> = EnumBits(flags.fold(0) { acc, f -> acc or (1 shl f.ordinal) })

        fun <E : Enum<E>> of(flags: Iterable<E>): EnumBits<E> = EnumBits(flags.fold(0) { acc, f -> acc or (1 shl f.ordinal) })
    }
}

inline fun <reified E : Enum<E>> EnumBits<E>.toList(): List<E> = enumEntries<E>().filter { it in this }

inline fun <reified E : Enum<E>> enumBitsOrNull(bits: Int): EnumBits<E>? =
    if (bits ushr enumEntries<E>().size == 0) EnumBits(bits) else null

inline fun <reified E : Enum<E>> allBits(): EnumBits<E> = EnumBits((1 shl enumEntries<E>().size) - 1)
