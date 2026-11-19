use candid::{CandidType, Nat};
use serde::Deserialize;

pub const WASM_PAGE_BYTES: u64 = 64 * 1024;

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimerStatus {
    pub name: String,
    pub period_ns: u64,
    pub last_run_ns: u64,
    pub runs: u64,
}

impl TimerStatus {
    pub fn is_overdue(&self, now_ns: u64, started_ns: u64) -> bool {
        let since = self.last_run_ns.max(started_ns);
        now_ns.saturating_sub(since) > self.period_ns.saturating_mul(2)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Status {
    pub version: String,
    pub cycles: Nat,
    pub stable_bytes: u64,
    pub timers: Vec<TimerStatus>,
    pub metrics: Vec<(String, u64)>,
    pub flags: Vec<String>,
}

impl Status {
    pub fn new(
        version: &str,
        cycles: u128,
        stable_pages: u64,
        timers: Vec<TimerStatus>,
        mut metrics: Vec<(String, u64)>,
        mut flags: Vec<String>,
    ) -> Self {
        metrics.sort_unstable();
        flags.sort_unstable();
        flags.dedup();
        Self {
            version: version.to_owned(),
            cycles: Nat::from(cycles),
            stable_bytes: stable_pages.saturating_mul(WASM_PAGE_BYTES),
            timers,
            metrics,
            flags,
        }
    }

    pub fn metric(&self, name: &str) -> Option<u64> {
        self.metrics
            .binary_search_by(|(k, _)| k.as_str().cmp(name))
            .ok()
            .map(|i| self.metrics[i].1)
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags
            .binary_search_by(|f| f.as_str().cmp(flag))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status {
        Status::new(
            "1.2.3",
            u128::MAX,
            3,
            vec![],
            vec![("zeta".into(), 1), ("alpha".into(), 2), ("mid".into(), 3)],
            vec!["b".into(), "a".into(), "b".into()],
        )
    }

    #[test]
    fn new_sorts_metrics_and_dedups_flags() {
        let s = status();
        let names: Vec<&str> = s.metrics.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["alpha", "mid", "zeta"]);
        assert_eq!(s.flags, ["a", "b"]);
    }

    #[test]
    fn new_converts_pages_and_cycles() {
        let s = status();
        assert_eq!(s.stable_bytes, 3 * 65536);
        assert_eq!(s.cycles, Nat::from(u128::MAX));
        assert_eq!(s.version, "1.2.3");
    }

    #[test]
    fn stable_bytes_saturate() {
        let s = Status::new("v", 0, u64::MAX, vec![], vec![], vec![]);
        assert_eq!(s.stable_bytes, u64::MAX);
    }

    #[test]
    fn metric_and_flag_lookup() {
        let s = status();
        assert_eq!(s.metric("mid"), Some(3));
        assert_eq!(s.metric("alpha"), Some(2));
        assert_eq!(s.metric("missing"), None);
        assert!(s.has_flag("a"));
        assert!(!s.has_flag("c"));
        let empty = Status::new("v", 0, 0, vec![], vec![], vec![]);
        assert_eq!(empty.metric(""), None);
        assert!(!empty.has_flag(""));
    }

    #[test]
    fn new_is_deterministic_regardless_of_input_order() {
        let a = Status::new(
            "v",
            1,
            1,
            vec![],
            vec![("b".into(), 1), ("a".into(), 2)],
            vec!["y".into(), "x".into()],
        );
        let b = Status::new(
            "v",
            1,
            1,
            vec![],
            vec![("a".into(), 2), ("b".into(), 1)],
            vec!["x".into(), "y".into()],
        );
        assert_eq!(a, b);
    }

    #[test]
    fn overdue_at_twice_the_period() {
        let t = TimerStatus {
            name: "drain".into(),
            period_ns: 100,
            last_run_ns: 1_000,
            runs: 1,
        };
        assert!(!t.is_overdue(1_200, 0));
        assert!(t.is_overdue(1_201, 0));
        assert!(!t.is_overdue(0, 0));
    }

    #[test]
    fn never_run_timer_measures_from_start() {
        let t = TimerStatus {
            name: "sync".into(),
            period_ns: 10,
            last_run_ns: 0,
            runs: 0,
        };
        assert!(!t.is_overdue(5_020, 5_000));
        assert!(t.is_overdue(5_021, 5_000));
    }

    #[test]
    fn overdue_saturates_on_huge_period() {
        let t = TimerStatus {
            name: "x".into(),
            period_ns: u64::MAX,
            last_run_ns: 0,
            runs: 0,
        };
        assert!(!t.is_overdue(u64::MAX, 0));
    }
}
