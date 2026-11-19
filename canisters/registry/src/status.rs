use crate::state::Published;
use crate::store::Stores;
use serde_bytes::ByteBuf;

pub fn metrics(st: &Stores) -> Vec<(String, u64)> {
    let published = st
        .state
        .get()
        .published
        .as_ref()
        .map_or(0, |p| p.leaf_count);
    vec![
        ("records".into(), st.records.len()),
        ("keys".into(), st.keys.len()),
        ("pending_keys".into(), st.pending.len()),
        ("outbox".into(), st.outbox.len()),
        ("tombstones".into(), st.tombstones.len()),
        ("published_leaf_count".into(), published),
    ]
}

pub fn flags(st: &Stores) -> Vec<String> {
    let s = st.state.get();
    [
        (cfg!(feature = "dev-attestation"), "dev_attestation"),
        (s.secret().is_none(), "challenge_pending"),
        (s.ca_root().is_none(), "ca_pending"),
        (!st.config.get().is_configured(), "unconfigured"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, f)| f.to_owned())
    .collect()
}

pub fn publish(st: &mut Stores, now: u64) -> Published {
    let roots = st.roots();
    let p = Published {
        leaf_count: roots.leaf_count,
        root: ByteBuf::from(roots.mmr_root.to_vec()),
        at_ns: now,
    };
    st.update_state(|s| s.published = Some(p.clone()));
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::with;

    #[test]
    fn fresh_canister_flags_everything_pending() {
        with(|st| {
            let f = flags(st);
            assert!(f.contains(&"challenge_pending".to_owned()));
            assert!(f.contains(&"ca_pending".to_owned()));
            assert!(f.contains(&"unconfigured".to_owned()));
            assert_eq!(
                f.contains(&"dev_attestation".to_owned()),
                cfg!(feature = "dev-attestation")
            );
            assert!(metrics(st).iter().all(|(_, v)| *v == 0));
        });
    }

    #[test]
    fn ready_canister_clears_flags() {
        with(|st| {
            crate::enroll::tests::ready(st);
            let f = flags(st);
            assert!(!f.contains(&"ca_pending".to_owned()));
            assert!(!f.contains(&"challenge_pending".to_owned()));
        });
    }

    #[test]
    fn metric_names_are_unique() {
        with(|st| {
            let mut names: Vec<String> = metrics(st).into_iter().map(|(n, _)| n).collect();
            let n = names.len();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), n);
        });
    }

    #[test]
    fn publish_records_current_root() {
        with(|st| {
            st.mmr().append(b"x").unwrap();
            let p = publish(st, 42);
            assert_eq!(p.leaf_count, 1);
            assert_eq!(p.at_ns, 42);
            assert_eq!(p.root.as_slice(), &st.roots().mmr_root);
            assert_eq!(st.state.get().published, Some(p));
            assert!(metrics(st).contains(&("published_leaf_count".into(), 1)));
        });
    }
}
