use crate::inspect::{MethodLimit, arg_allowed};
use crate::status::{Status, TimerStatus};
use crate::timers::{TimerBook, TimerSpec, validate_specs};
use std::cell::RefCell;

thread_local! {
    static TIMERS: RefCell<TimerBook> = const { RefCell::new(TimerBook::new()) };
}

pub fn set_certified_root(root: &[u8; 32]) {
    ic_cdk::api::certified_data_set(root);
}

pub fn certificate() -> Vec<u8> {
    ic_cdk::api::data_certificate().unwrap_or_default()
}

pub fn inspect(limits: &[MethodLimit], default_max: Option<usize>) {
    let method = ic_cdk::api::msg_method_name();
    if arg_allowed(limits, default_max, &method, ic0::msg_arg_data_size()) {
        ic_cdk::api::accept_message();
    }
}

pub fn start_timers(specs: &'static [TimerSpec]) {
    if let Err(e) = validate_specs(specs) {
        ic_cdk::trap(format!("invalid timer specs: {e:?}"));
    }
    for spec in specs {
        TIMERS.with_borrow_mut(|b| b.register(spec.name, spec.period));
        ic_cdk_timers::set_timer_interval_serial(spec.period, async move || {
            (spec.tick)().await;
            TIMERS.with_borrow_mut(|b| b.record_run(spec.name, ic_cdk::api::time()));
        });
    }
}

pub fn timer_statuses() -> Vec<TimerStatus> {
    TIMERS.with_borrow(TimerBook::snapshot)
}

pub fn status(version: &str, metrics: Vec<(String, u64)>, flags: Vec<String>) -> Status {
    Status::new(
        version,
        ic_cdk::api::canister_cycle_balance(),
        ic_cdk::api::stable_size(),
        timer_statuses(),
        metrics,
        flags,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_statuses_start_empty() {
        assert!(timer_statuses().is_empty());
    }

    #[test]
    fn timer_statuses_reflect_book() {
        TIMERS.with_borrow_mut(|b| {
            b.register("b", core::time::Duration::from_secs(1));
            b.register("a", core::time::Duration::from_secs(2));
            b.record_run("a", 9);
        });
        let s = timer_statuses();
        assert_eq!(
            s.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!((s[0].last_run_ns, s[0].runs), (9, 1));
    }
}
