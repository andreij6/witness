use crate::config::{BACKOFF_BASE_NS, BACKOFF_MAX_SHIFT, ENROLL_FREE, ENROLL_WINDOW_NS};
use witness_attest::policy::DAY_NS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnrollWindow {
    pub start_ns: u64,
    pub count: u64,
    pub last_ns: u64,
}

impl EnrollWindow {
    pub fn to_bytes(self) -> [u8; 24] {
        let mut out = [0u8; 24];
        out[..8].copy_from_slice(&self.start_ns.to_be_bytes());
        out[8..16].copy_from_slice(&self.count.to_be_bytes());
        out[16..].copy_from_slice(&self.last_ns.to_be_bytes());
        out
    }

    pub fn from_bytes(b: &[u8; 24]) -> Self {
        let at = |i: usize| u64::from_be_bytes(b[i..i + 8].try_into().unwrap_or_default());
        Self {
            start_ns: at(0),
            count: at(8),
            last_ns: at(16),
        }
    }

    pub fn admit(prev: Option<Self>, now_ns: u64, backoff: bool) -> Result<Self, u64> {
        let w = prev
            .filter(|w| now_ns.saturating_sub(w.start_ns) < ENROLL_WINDOW_NS)
            .unwrap_or(Self {
                start_ns: now_ns,
                count: 0,
                last_ns: 0,
            });
        if w.count >= ENROLL_FREE {
            let wait = if backoff {
                let shift = (w.count - ENROLL_FREE).min(BACKOFF_MAX_SHIFT);
                (BACKOFF_BASE_NS << shift).saturating_sub(now_ns.saturating_sub(w.last_ns))
            } else {
                (w.start_ns + ENROLL_WINDOW_NS).saturating_sub(now_ns)
            };
            if wait > 0 {
                return Err(wait);
            }
        }
        Ok(Self {
            count: w.count + 1,
            last_ns: now_ns,
            ..w
        })
    }

    pub fn release(self, start_ns: u64) -> Self {
        if self.start_ns == start_ns {
            Self {
                count: self.count.saturating_sub(1),
                ..self
            }
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Daily {
    pub day: u64,
    pub count: u64,
}

impl Daily {
    pub fn to_bytes(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.day.to_be_bytes());
        out[8..].copy_from_slice(&self.count.to_be_bytes());
        out
    }

    pub fn from_bytes(b: &[u8; 16]) -> Self {
        let at = |i: usize| u64::from_be_bytes(b[i..i + 8].try_into().unwrap_or_default());
        Self {
            day: at(0),
            count: at(8),
        }
    }

    pub fn admit(prev: Option<Self>, now_ns: u64, limit: u64) -> Result<Self, u64> {
        let day = now_ns / DAY_NS;
        let count = prev.filter(|d| d.day == day).map_or(0, |d| d.count);
        if count >= limit {
            return Err((day + 1) * DAY_NS - now_ns);
        }
        Ok(Self {
            day,
            count: count + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_763_510_400_000_000_000;

    fn fill(backoff: bool) -> EnrollWindow {
        let mut w = None;
        for i in 0..ENROLL_FREE {
            w = Some(EnrollWindow::admit(w, T0 + i, backoff).unwrap());
        }
        w.unwrap()
    }

    #[test]
    fn enroll_bytes_round_trip() {
        let w = EnrollWindow {
            start_ns: 1,
            count: u64::MAX,
            last_ns: 3,
        };
        assert_eq!(EnrollWindow::from_bytes(&w.to_bytes()), w);
        assert_eq!(w.to_bytes(), w.to_bytes());
    }

    #[test]
    fn enroll_admits_free_quota_then_hard_blocks() {
        let w = fill(false);
        assert_eq!(w.count, ENROLL_FREE);
        assert_eq!(w.start_ns, T0);
        let wait = EnrollWindow::admit(Some(w), T0 + 10, false).unwrap_err();
        assert_eq!(wait, ENROLL_WINDOW_NS - 10);
        let fresh = EnrollWindow::admit(Some(w), T0 + ENROLL_WINDOW_NS, false).unwrap();
        assert_eq!((fresh.start_ns, fresh.count), (T0 + ENROLL_WINDOW_NS, 1));
    }

    #[test]
    fn enroll_backoff_grows_and_caps() {
        let mut w = fill(true);
        let mut now = T0 + 100;
        for k in 0..14u64 {
            let gap = BACKOFF_BASE_NS << k.min(BACKOFF_MAX_SHIFT);
            let early = EnrollWindow::admit(Some(w), now + 1, true).unwrap_err();
            assert!(early > 0 && early <= gap);
            now = w.last_ns + gap;
            if now.saturating_sub(w.start_ns) >= ENROLL_WINDOW_NS {
                break;
            }
            w = EnrollWindow::admit(Some(w), now, true).unwrap();
            assert_eq!(w.last_ns, now);
        }
    }

    #[test]
    fn enroll_never_blocks_backoff_forever() {
        let w = EnrollWindow {
            start_ns: T0,
            count: u64::MAX - 1,
            last_ns: T0,
        };
        let wait = EnrollWindow::admit(Some(w), T0 + 1, true).unwrap_err();
        assert!(wait <= BACKOFF_BASE_NS << BACKOFF_MAX_SHIFT);
        assert!(
            EnrollWindow::admit(Some(w), T0 + (BACKOFF_BASE_NS << BACKOFF_MAX_SHIFT), true).is_ok()
        );
    }

    #[test]
    fn enroll_release_only_same_window() {
        let w = fill(false);
        assert_eq!(w.release(T0).count, ENROLL_FREE - 1);
        assert_eq!(w.release(T0 + 1), w);
        let zero = EnrollWindow { count: 0, ..w };
        assert_eq!(zero.release(T0).count, 0);
    }

    #[test]
    fn enroll_handles_clock_before_window() {
        let w = EnrollWindow {
            start_ns: T0,
            count: 1,
            last_ns: T0,
        };
        assert_eq!(
            EnrollWindow::admit(Some(w), T0 - 5, false).unwrap().count,
            2
        );
    }

    #[test]
    fn daily_bytes_round_trip() {
        let d = Daily { day: 9, count: 8 };
        assert_eq!(Daily::from_bytes(&d.to_bytes()), d);
    }

    #[test]
    fn daily_counts_and_resets() {
        let mut d = None;
        for i in 1..=3 {
            d = Some(Daily::admit(d, T0, 3).unwrap());
            assert_eq!(d.unwrap().count, i);
        }
        let wait = Daily::admit(d, T0, 3).unwrap_err();
        assert_eq!((T0 + wait) % DAY_NS, 0);
        assert!(wait <= DAY_NS);
        let next = Daily::admit(d, T0 + wait, 3).unwrap();
        assert_eq!(next.count, 1);
        assert_eq!(next.day, d.unwrap().day + 1);
    }

    #[test]
    fn daily_zero_limit_blocks() {
        assert!(Daily::admit(None, 0, 0).is_err());
        assert_eq!(Daily::admit(None, 0, 0).unwrap_err(), DAY_NS);
    }

    #[test]
    fn daily_is_deterministic() {
        assert_eq!(Daily::admit(None, T0, 5), Daily::admit(None, T0, 5));
    }
}
