package app.witness.core.model

import kotlin.jvm.JvmInline

@JvmInline
value class CaptureId private constructor(val value: Long) : Comparable<CaptureId> {
    val schema: Int get() = (value ushr RANDOM_BITS).toInt()
    val random: Long get() = value and RANDOM_MASK

    override fun compareTo(other: CaptureId): Int = value.compareTo(other.value)

    companion object {
        const val BITS: Int = 61
        const val RANDOM_BITS: Int = 56
        const val SCHEMA_BITS: Int = 5
        const val RANDOM_MASK: Long = (1L shl RANDOM_BITS) - 1
        const val MAX: Long = (1L shl BITS) - 1

        fun of(random: Long, schema: Int): CaptureId? =
            if (random !in 0..RANDOM_MASK || schema !in 0 until (1 shl SCHEMA_BITS)) null
            else CaptureId((schema.toLong() shl RANDOM_BITS) or random)

        fun fromRandomBytes(random: ByteArray, schema: Int): CaptureId? {
            if (random.size != 7) return null
            var v = 0L
            for (b in random) v = (v shl 8) or (b.toLong() and 0xff)
            return of(v, schema)
        }

        fun fromLong(value: Long): CaptureId? = if (value in 0..MAX) CaptureId(value) else null
    }
}

@JvmInline
value class RecordId private constructor(val text: String) : Comparable<RecordId> {
    fun toBytes(): ByteArray = Crockford.decode(text, SIZE)

    fun grouped(): String = Crockford.group(text)

    override fun compareTo(other: RecordId): Int = text.compareTo(other.text)

    override fun toString(): String = text

    companion object {
        const val SIZE: Int = 10
        const val TEXT_LENGTH: Int = 16
        const val GROUPED_LENGTH: Int = 19

        fun fromBytes(bytes: ByteArray): RecordId {
            require(bytes.size == SIZE) { "record id must be $SIZE bytes" }
            return RecordId(Crockford.encode(bytes))
        }

        fun fromDigest(sha256: ByteArray): RecordId {
            require(sha256.size >= SIZE) { "digest too short" }
            return RecordId(Crockford.encode(sha256.copyOf(SIZE)))
        }

        fun parse(text: String): RecordId = RecordId(Crockford.encode(Crockford.decode(text, SIZE)))

        fun parseOrNull(text: String): RecordId? = Crockford.canonicalOrNull(text, SIZE)?.let(::RecordId)
    }
}

@JvmInline
value class KeyId private constructor(val text: String) : Comparable<KeyId> {
    fun toBytes(): ByteArray = Crockford.decode(text, SIZE)

    fun short(): String = text.substring(0, SHORT_LENGTH)

    override fun compareTo(other: KeyId): Int = text.compareTo(other.text)

    override fun toString(): String = text

    companion object {
        const val SIZE: Int = 16
        const val TEXT_LENGTH: Int = 26
        const val SHORT_LENGTH: Int = 8

        fun fromBytes(bytes: ByteArray): KeyId {
            require(bytes.size == SIZE) { "key id must be $SIZE bytes" }
            return KeyId(Crockford.encode(bytes))
        }

        fun fromDigest(sha256: ByteArray): KeyId {
            require(sha256.size >= SIZE) { "digest too short" }
            return KeyId(Crockford.encode(sha256.copyOf(SIZE)))
        }

        fun parse(text: String): KeyId = KeyId(Crockford.encode(Crockford.decode(text, SIZE)))

        fun parseOrNull(text: String): KeyId? = Crockford.canonicalOrNull(text, SIZE)?.let(::KeyId)
    }
}

@JvmInline
value class CaseId private constructor(val text: String) : Comparable<CaseId> {
    val shard: Int get() = toBytes()[0].toInt() and 0xff

    fun toBytes(): ByteArray = Crockford.decode(text, SIZE)

    fun grouped(): String = Crockford.group(text)

    override fun compareTo(other: CaseId): Int = text.compareTo(other.text)

    override fun toString(): String = text

    companion object {
        const val SIZE: Int = 16
        const val RANDOM_SIZE: Int = 15
        const val TEXT_LENGTH: Int = 26

        fun of(shard: Int, random: ByteArray): CaseId {
            require(shard in 0..255) { "shard out of range" }
            require(random.size == RANDOM_SIZE) { "case id random must be $RANDOM_SIZE bytes" }
            val bytes = ByteArray(SIZE)
            bytes[0] = shard.toByte()
            random.copyInto(bytes, 1)
            return CaseId(Crockford.encode(bytes))
        }

        fun fromBytes(bytes: ByteArray): CaseId {
            require(bytes.size == SIZE) { "case id must be $SIZE bytes" }
            return CaseId(Crockford.encode(bytes))
        }

        fun parse(text: String): CaseId = CaseId(Crockford.encode(Crockford.decode(text, SIZE)))

        fun parseOrNull(text: String): CaseId? = Crockford.canonicalOrNull(text, SIZE)?.let(::CaseId)
    }
}

@JvmInline
value class LocalCaseId(val value: Long) : Comparable<LocalCaseId> {
    override fun compareTo(other: LocalCaseId): Int = value.compareTo(other.value)
}
