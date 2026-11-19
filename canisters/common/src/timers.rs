use crate::status::TimerStatus;
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;

pub type Tick = Pin<Box<dyn Future<Output = ()>>>;

#[derive(Clone, Copy)]
pub struct TimerSpec {
    pub name: &'static str,
    pub period: Duration,
    pub tick: fn() -> Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecError {
    EmptyName,
    DuplicateName(&'static str),
    ZeroPeriod(&'static str),
    PeriodTooLong(&'static str),
}

pub fn validate_specs(specs: &[TimerSpec]) -> Result<(), SpecError> {
    for (i, s) in specs.iter().enumerate() {
        if s.name.is_empty() {
            return Err(SpecError::EmptyName);
        }
        if s.period.is_zero() {
            return Err(SpecError::ZeroPeriod(s.name));
        }
        if u64::try_from(s.period.as_nanos()).is_err() {
            return Err(SpecError::PeriodTooLong(s.name));
        }
        if specs[..i].iter().any(|p| p.name == s.name) {
            return Err(SpecError::DuplicateName(s.name));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimerBook {
    entries: Vec<TimerStatus>,
}

impl TimerBook {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn find(&self, name: &str) -> Result<usize, usize> {
        self.entries.binary_search_by(|e| e.name.as_str().cmp(name))
    }

    pub fn register(&mut self, name: &str, period: Duration) {
        let period_ns = u64::try_from(period.as_nanos()).unwrap_or(u64::MAX);
        match self.find(name) {
            Ok(i) => self.entries[i].period_ns = period_ns,
            Err(i) => self.entries.insert(
                i,
                TimerStatus {
                    name: name.to_owned(),
                    period_ns,
                    last_run_ns: 0,
                    runs: 0,
                },
            ),
        }
    }

    pub fn record_run(&mut self, name: &str, now_ns: u64) -> bool {
        match self.find(name) {
            Ok(i) => {
                let e = &mut self.entries[i];
                e.last_run_ns = e.last_run_ns.max(now_ns);
                e.runs = e.runs.saturating_add(1);
                true
            }
            Err(_) => false,
        }
    }

    pub fn get(&self, name: &str) -> Option<&TimerStatus> {
        self.find(name).ok().map(|i| &self.entries[i])
    }

    pub fn snapshot(&self) -> Vec<TimerStatus> {
        self.entries.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop() -> Tick {
        Box::pin(async {})
    }

    fn spec(name: &'static str, secs: u64) -> TimerSpec {
        TimerSpec {
            name,
            period: Duration::from_secs(secs),
            tick: noop,
        }
    }

    #[test]
    fn valid_specs_pass() {
        assert_eq!(validate_specs(&[]), Ok(()));
        assert_eq!(
            validate_specs(&[spec("drain", 5), spec("sync", 86_400)]),
            Ok(())
        );
    }

    #[test]
    fn invalid_specs_are_rejected() {
        assert_eq!(validate_specs(&[spec("", 1)]), Err(SpecError::EmptyName));
        assert_eq!(
            validate_specs(&[spec("a", 0)]),
            Err(SpecError::ZeroPeriod("a"))
        );
        assert_eq!(
            validate_specs(&[spec("a", 1), spec("b", 1), spec("a", 2)]),
            Err(SpecError::DuplicateName("a"))
        );
        let long = TimerSpec {
            name: "long",
            period: Duration::MAX,
            tick: noop,
        };
        assert_eq!(
            validate_specs(&[long]),
            Err(SpecError::PeriodTooLong("long"))
        );
    }

    #[test]
    fn register_keeps_entries_sorted_and_idempotent() {
        let mut b = TimerBook::new();
        b.register("sync", Duration::from_secs(10));
        b.register("drain", Duration::from_secs(5));
        b.register("expiry", Duration::from_secs(600));
        b.register("drain", Duration::from_secs(6));
        let names: Vec<String> = b.snapshot().into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["drain", "expiry", "sync"]);
        assert_eq!(b.get("drain").map(|t| t.period_ns), Some(6_000_000_000));
    }

    #[test]
    fn register_preserves_run_history() {
        let mut b = TimerBook::new();
        b.register("drain", Duration::from_secs(5));
        b.record_run("drain", 77);
        b.register("drain", Duration::from_secs(9));
        let t = b.get("drain").cloned();
        assert_eq!(t.as_ref().map(|t| (t.last_run_ns, t.runs)), Some((77, 1)));
    }

    #[test]
    fn record_run_updates_known_timer() {
        let mut b = TimerBook::new();
        b.register("drain", Duration::from_secs(5));
        assert!(b.record_run("drain", 100));
        assert!(b.record_run("drain", 200));
        let t = b.get("drain").cloned().unwrap_or_else(|| panic!("missing"));
        assert_eq!((t.last_run_ns, t.runs), (200, 2));
    }

    #[test]
    fn record_run_never_moves_backwards() {
        let mut b = TimerBook::new();
        b.register("drain", Duration::from_secs(5));
        b.record_run("drain", 500);
        b.record_run("drain", 100);
        assert_eq!(b.get("drain").map(|t| t.last_run_ns), Some(500));
    }

    #[test]
    fn record_run_ignores_unknown_timer() {
        let mut b = TimerBook::new();
        assert!(!b.record_run("ghost", 1));
        assert!(b.snapshot().is_empty());
        assert_eq!(b.get("ghost"), None);
    }

    #[test]
    fn runs_saturate() {
        let mut b = TimerBook::new();
        b.register("t", Duration::from_nanos(1));
        b.entries[0].runs = u64::MAX;
        b.record_run("t", 1);
        assert_eq!(b.get("t").map(|t| t.runs), Some(u64::MAX));
    }

    #[test]
    fn huge_period_saturates_in_book() {
        let mut b = TimerBook::new();
        b.register("t", Duration::MAX);
        assert_eq!(b.get("t").map(|t| t.period_ns), Some(u64::MAX));
    }

    #[test]
    fn snapshot_is_deterministic() {
        let build = |order: &[&str]| {
            let mut b = TimerBook::new();
            for n in order {
                b.register(n, Duration::from_secs(1));
            }
            b.snapshot()
        };
        assert_eq!(build(&["c", "a", "b"]), build(&["b", "c", "a"]));
    }

    #[test]
    fn tick_future_runs_to_completion() {
        let f = (spec("x", 1).tick)();
        let waker = core::task::Waker::noop();
        let mut cx = core::task::Context::from_waker(waker);
        let mut f = f;
        assert!(f.as_mut().poll(&mut cx).is_ready());
    }
}
