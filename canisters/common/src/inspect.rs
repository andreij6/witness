#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodLimit {
    pub method: &'static str,
    pub max_arg_bytes: usize,
}

pub const fn limit(method: &'static str, max_arg_bytes: usize) -> MethodLimit {
    MethodLimit {
        method,
        max_arg_bytes,
    }
}

pub fn arg_allowed(
    limits: &[MethodLimit],
    default_max: Option<usize>,
    method: &str,
    arg_len: usize,
) -> bool {
    limits
        .iter()
        .find(|l| l.method == method)
        .map(|l| l.max_arg_bytes)
        .or(default_max)
        .is_some_and(|max| arg_len <= max)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: &[MethodLimit] = &[limit("submit", 1000), limit("erase_mine", 200)];

    #[test]
    fn listed_methods_use_their_limit() {
        assert!(arg_allowed(LIMITS, None, "submit", 0));
        assert!(arg_allowed(LIMITS, None, "submit", 1000));
        assert!(!arg_allowed(LIMITS, None, "submit", 1001));
        assert!(arg_allowed(LIMITS, Some(10_000), "erase_mine", 200));
        assert!(!arg_allowed(LIMITS, Some(10_000), "erase_mine", 201));
    }

    #[test]
    fn unlisted_methods_fall_back_to_default() {
        assert!(arg_allowed(LIMITS, Some(50), "status", 50));
        assert!(!arg_allowed(LIMITS, Some(50), "status", 51));
    }

    #[test]
    fn unlisted_methods_rejected_without_default() {
        assert!(!arg_allowed(LIMITS, None, "status", 0));
        assert!(!arg_allowed(&[], None, "", 0));
    }

    #[test]
    fn method_match_is_exact() {
        assert!(!arg_allowed(LIMITS, None, "Submit", 1));
        assert!(!arg_allowed(LIMITS, None, "submit ", 1));
        assert!(!arg_allowed(LIMITS, None, "sub", 1));
    }

    #[test]
    fn first_entry_wins_on_duplicates() {
        let dup = [limit("m", 5), limit("m", 500)];
        assert!(!arg_allowed(&dup, None, "m", 6));
    }

    #[test]
    fn huge_lengths_are_rejected() {
        assert!(!arg_allowed(LIMITS, Some(usize::MAX - 1), "x", usize::MAX));
        assert!(arg_allowed(LIMITS, Some(usize::MAX), "x", usize::MAX));
    }
}
