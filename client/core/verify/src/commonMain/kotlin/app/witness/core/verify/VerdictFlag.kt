package app.witness.core.verify

enum class VerdictFlag(val bit: Int) {
    KEY_REVOKED_AFTER_ANCHOR(0),
    LATE_ANCHOR(1),
    CLOCK_MISMATCH(2),
    COULDNT_COMPARE(3),
    NO_PUBLIC_REFERENCE(4),
    DEV_ATTESTATION(5),
    TRUNCATED_CANDIDATES(6);

    val mask: Int get() = 1 shl bit

    companion object {
        val ALL_BITS: Int = entries.fold(0) { acc, f -> acc or f.mask }

        fun decode(bits: Int): Set<VerdictFlag> {
            if (bits and ALL_BITS == 0) return emptySet()
            return entries.filterTo(LinkedHashSet()) { bits and it.mask != 0 }
        }

        fun encode(flags: Set<VerdictFlag>): Int = flags.fold(0) { acc, f -> acc or f.mask }
    }
}
