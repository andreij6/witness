package app.witness.core.model

enum class Verdict {
    ORIGINAL,
    MATCHES_WITH_CHANGES,
    ALTERED,
    NO_RECORD,
    SEALED_UNANCHORED;

    val isTransient: Boolean get() = this == SEALED_UNANCHORED

    val isSealedMatch: Boolean get() = this == ORIGINAL || this == MATCHES_WITH_CHANGES || this == SEALED_UNANCHORED
}

enum class VerdictFlag {
    KEY_REVOKED_AFTER_ANCHOR,
    LATE_ANCHOR,
    CLOCK_MISMATCH,
    COULDNT_COMPARE,
    NO_PUBLIC_REFERENCE,
    DEV_ATTESTATION,
    TRUNCATED_CANDIDATES,
}

typealias VerdictFlags = EnumBits<VerdictFlag>

enum class VerdictReason {
    SEAL_VERIFIED,
    SEAL_BROKEN,
    UNVERIFIED_CREDENTIALS,
    OTHER_CREDENTIALS,
    KEY_NOT_TRUSTED,
    KEY_NOT_TRUSTED_AT_ANCHOR,
    EXACT_ASSET_HASH,
    EXACT_PIXEL_HASH,
    WATERMARK_FOUND,
    WATERMARK_UNAVAILABLE,
    FOREIGN_WATERMARK,
    UNCONFIRMED,
    NO_CANDIDATES,
    UNDECODABLE,
    INDEX_PENDING,
}

typealias VerdictReasons = EnumBits<VerdictReason>
