mod candidates;
mod confirm;
mod credentials;
mod exact;
mod judge;
mod options;
mod oracle;
mod query;
mod verdict;

#[cfg(any(test, feature = "testkit"))]
pub mod testkit;

#[cfg(test)]
mod tests;

pub use candidates::{MAX_CANDIDATES, MAX_PROBE_HITS, NO_DISTANCE};
pub use confirm::{
    FOREIGN_MAX_ERRORS, FOREIGN_MIN_DISTANCE, FULL_COMPARED_Q16, GRID_CHANGED_DISTANCE,
    MIN_COMPARED_Q16, TAU_PIXEL_Q16,
};
pub use judge::judge;
pub use options::{ANDROID_TOP_K, MAX_TOP_K, Options, WEB_TOP_K};
pub use oracle::{
    CLOCK_TOLERANCE_MS, LATE_ANCHOR_MS, MAX_DERIVATIVES, MAX_EXACT, MAX_WATERMARK, Oracle, PdqHit,
    PdqProbe, Record, clock_flags,
};
pub use query::{MAX_QUERY_RECTS, MAX_QUERY_VIEWS, QUERY_LONG_EDGE};
pub use verdict::{
    Anchor, Evidence, Q16_ONE, Reason, Reasons, Reference, Region, VERDICT_V, Verdict,
};
