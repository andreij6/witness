use crate::net::{Ids, R};
use serde_json::{Map, Value, json};
use std::time::{Duration, Instant};

const POLL_EVERY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub ms: u128,
    pub ok: bool,
    pub detail: Value,
}

#[derive(Debug, Default)]
pub struct Steps {
    entries: Vec<Entry>,
    notes: Map<String, Value>,
}

impl Steps {
    fn record<T>(
        &mut self,
        name: &str,
        f: impl FnOnce() -> R<T>,
        detail: impl Fn(&T) -> Value,
    ) -> R<T> {
        let start = Instant::now();
        let r = f();
        self.entries.push(Entry {
            name: name.to_owned(),
            ms: start.elapsed().as_millis(),
            ok: r.is_ok(),
            detail: r.as_ref().map_or_else(|e| json!(e), &detail),
        });
        r.map_err(|e| format!("{name}: {e}"))
    }

    pub fn step<T>(&mut self, name: &str, f: impl FnOnce() -> R<T>) -> R<T> {
        self.record(name, f, |_| Value::Null)
    }

    pub fn check(&mut self, name: &str, f: impl FnOnce() -> R<Value>) -> R<Value> {
        self.record(name, f, Value::clone)
    }

    pub fn note(&mut self, key: &str, v: Value) {
        self.notes.insert(key.to_owned(), v);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn total_ms(&self) -> u128 {
        self.entries.iter().map(|e| e.ms).sum()
    }

    pub fn table(&self) -> String {
        let width = self.entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
        let mut out = String::new();
        for e in &self.entries {
            let mark = if e.ok { "ok  " } else { "FAIL" };
            let detail = if e.detail.is_null() {
                String::new()
            } else {
                e.detail.to_string()
            };
            out.push_str(&format!(
                "{mark} {:>7} ms  {:<width$}  {detail}\n",
                e.ms, e.name
            ));
        }
        out.push_str(&format!("total {} ms", self.total_ms()));
        out
    }

    pub fn report(&self, ids: &Ids, failure: Option<&String>) -> Value {
        json!({
            "ok": failure.is_none() && self.entries.iter().all(|e| e.ok),
            "failure": failure,
            "gateway": ids.gateway,
            "canisters": {
                "registry": ids.registry.to_text(),
                "index_0": ids.index.to_text(),
                "pages": ids.pages.to_text(),
                "media_0": ids.media.to_text(),
                "web": ids.web.to_text(),
            },
            "notes": self.notes,
            "total_ms": self.total_ms(),
            "steps": self.entries.iter().map(|e| json!({
                "name": e.name, "ms": e.ms, "ok": e.ok, "detail": e.detail,
            })).collect::<Vec<_>>(),
        })
    }
}

pub fn poll<T>(timeout: Duration, mut f: impl FnMut() -> R<Option<T>>) -> R<T> {
    let start = Instant::now();
    loop {
        if let Some(v) = f()? {
            return Ok(v);
        }
        if start.elapsed() >= timeout {
            return Err(format!("timed out after {} s", timeout.as_secs()));
        }
        std::thread::sleep(POLL_EVERY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn ids() -> Ids {
        Ids {
            gateway: "http://127.0.0.1:8090".into(),
            registry: Principal::anonymous(),
            index: Principal::anonymous(),
            pages: Principal::anonymous(),
            media: Principal::anonymous(),
            web: Principal::anonymous(),
            ca_root: None,
        }
    }

    #[test]
    fn records_success_failure_and_details() {
        let mut s = Steps::default();
        assert_eq!(s.step("a", || Ok(3)).unwrap(), 3);
        assert_eq!(
            s.check("b", || Ok(json!({"x": 1}))).unwrap(),
            json!({"x": 1})
        );
        let e = s.step::<()>("c", || Err("boom".into())).unwrap_err();
        assert_eq!(e, "c: boom");
        assert_eq!(s.len(), 3);
        let flags: Vec<bool> = s.entries.iter().map(|e| e.ok).collect();
        assert_eq!(flags, vec![true, true, false]);
        assert_eq!(s.entries[0].detail, Value::Null);
        assert_eq!(s.entries[1].detail, json!({"x": 1}));
        assert_eq!(s.entries[2].detail, json!("boom"));
        assert!(s.table().contains("FAIL"));
        assert!(s.table().ends_with(&format!("total {} ms", s.total_ms())));
    }

    #[test]
    fn report_reflects_outcome_and_notes() {
        let mut s = Steps::default();
        s.step("a", || Ok(())).unwrap();
        s.note("k", json!(1));
        let r = s.report(&ids(), None);
        assert_eq!(r["ok"], json!(true));
        assert_eq!(r["notes"]["k"], json!(1));
        assert_eq!(r["steps"].as_array().unwrap().len(), 1);
        let bad = s.report(&ids(), Some(&"x".to_owned()));
        assert_eq!(bad["ok"], json!(false));
        assert_eq!(bad["failure"], json!("x"));
    }

    #[test]
    fn empty_steps_report_ok() {
        let s = Steps::default();
        assert_eq!(s.len(), 0);
        assert_eq!(s.table(), "total 0 ms");
        assert_eq!(s.report(&ids(), None)["ok"], json!(true));
    }

    #[test]
    fn poll_returns_first_value_or_error() {
        let mut n = 0;
        assert_eq!(
            poll(Duration::from_secs(5), || {
                n += 1;
                Ok((n == 2).then_some(n))
            })
            .unwrap(),
            2
        );
        assert_eq!(
            poll::<()>(Duration::from_secs(5), || Err("bad".into())).unwrap_err(),
            "bad"
        );
        assert!(
            poll::<()>(Duration::ZERO, || Ok(None))
                .unwrap_err()
                .contains("timed out")
        );
    }
}
