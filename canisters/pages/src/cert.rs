use crate::critbit::{Hash, Key};
use ic_certification::{HashTree, fork, labeled, labeled_hash, leaf, pruned};
use ic_http_certification::{
    DefaultCelBuilder, DefaultResponseCertification, DefaultResponseOnlyCelExpression,
};
use std::sync::LazyLock;
use witness_format::crypto::sha256;

pub const ROOT_LABEL: &[u8] = b"http_expr";
pub const EXACT: &[u8] = b"<$>";
pub const WILDCARD: &[u8] = b"<*>";

pub struct Cel {
    pub def: DefaultResponseOnlyCelExpression<'static>,
    pub text: String,
    pub hash: Hash,
}

pub static CEL: LazyLock<Cel> = LazyLock::new(|| {
    let def = DefaultCelBuilder::response_only_certification()
        .with_response_certification(DefaultResponseCertification::response_header_exclusions(
            Vec::<&str>::new(),
        ))
        .build();
    let text = def.to_string();
    let hash = sha256(text.as_bytes());
    Cel { def, text, hash }
});

pub fn entry(resp: &Hash) -> HashTree {
    labeled(
        CEL.hash.to_vec(),
        labeled(b"".to_vec(), labeled(resp.to_vec(), leaf(Vec::new()))),
    )
}

pub fn exact(resp: &Hash) -> HashTree {
    labeled(EXACT, entry(resp))
}

pub fn wildcard(resp: &Hash) -> HashTree {
    labeled(WILDCARD, entry(resp))
}

pub fn record_leaf(key: &Key, resp: &Hash, full: bool) -> HashTree {
    let sub = exact(resp);
    labeled(key.to_vec(), if full { sub } else { pruned(sub.digest()) })
}

pub fn record_leaf_hash(key: &Key, resp: &Hash) -> Hash {
    labeled_hash(key, &exact(resp).digest())
}

pub fn forks(mut nodes: Vec<HashTree>) -> HashTree {
    match nodes.len() {
        0 => ic_certification::empty(),
        1 => nodes.pop().expect("one node"),
        n => {
            let right = nodes.split_off(n / 2);
            fork(forks(nodes), forks(right))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_certification::{LookupResult, SubtreeLookupResult, empty};
    use ic_http_certification::{
        CERTIFICATE_EXPRESSION_HEADER_NAME, HttpCertification, HttpCertificationPath,
        HttpCertificationTree, HttpCertificationTreeEntry, HttpResponse, StatusCode,
    };

    fn response(body: &'static [u8]) -> HttpResponse<'static> {
        HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_headers(vec![(
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                CEL.text.clone(),
            )])
            .with_body(body)
            .build()
    }

    #[test]
    fn cel_is_stable_and_certifies_all_headers() {
        assert_eq!(CEL.hash, sha256(CEL.text.as_bytes()));
        assert!(CEL.text.contains("response_certification"));
        assert!(CEL.text.contains("no_request_certification"));
        assert!(
            CEL.text.contains("certified_response_headers")
                || CEL.text.contains("response_header_exclusions")
        );
    }

    #[test]
    fn exact_entry_matches_reference_tree_layout() {
        let resp = response(b"hello");
        let cert = HttpCertification::response_only(&CEL.def, &resp, None).unwrap();
        let rh = ic_http_certification::response_hash(&resp, &CEL.def.response, None);
        let mut reference = HttpCertificationTree::default();
        let entry_ref =
            HttpCertificationTreeEntry::new(HttpCertificationPath::exact("/p/abc"), cert);
        reference.insert(&entry_ref);
        let mine = labeled(
            ROOT_LABEL,
            labeled(b"p".to_vec(), labeled(b"abc".to_vec(), exact(&rh))),
        );
        assert_eq!(mine.digest(), reference.root_hash());
    }

    #[test]
    fn wildcard_entry_matches_reference_tree_layout() {
        let resp = response(b"nope");
        let cert = HttpCertification::response_only(&CEL.def, &resp, None).unwrap();
        let rh = ic_http_certification::response_hash(&resp, &CEL.def.response, None);
        let mut reference = HttpCertificationTree::default();
        reference.insert(&HttpCertificationTreeEntry::new(
            HttpCertificationPath::wildcard(""),
            cert,
        ));
        let mine = labeled(ROOT_LABEL, wildcard(&rh));
        assert_eq!(mine.digest(), reference.root_hash());
    }

    #[test]
    fn record_leaf_hash_matches_tree_digest_both_ways() {
        let key = *b"0123456789ABCDEF";
        let resp = [9u8; 32];
        assert_eq!(
            record_leaf(&key, &resp, true).digest(),
            record_leaf_hash(&key, &resp)
        );
        assert_eq!(
            record_leaf(&key, &resp, false).digest(),
            record_leaf_hash(&key, &resp)
        );
        assert_ne!(
            record_leaf_hash(&key, &[8u8; 32]),
            record_leaf_hash(&key, &resp)
        );
    }

    #[test]
    fn pruned_record_leaf_hides_entry() {
        let key = *b"0123456789ABCDEF";
        let t = record_leaf(&key, &[1u8; 32], false);
        assert_eq!(
            t.lookup_subtree([key.as_slice(), EXACT]),
            SubtreeLookupResult::Unknown
        );
        let full = record_leaf(&key, &[1u8; 32], true);
        assert!(matches!(
            full.lookup_subtree([key.as_slice(), EXACT]),
            SubtreeLookupResult::Found(_)
        ));
    }

    #[test]
    fn forks_keeps_sorted_order_for_lookups() {
        let labels: Vec<Vec<u8>> = (0u8..7).map(|i| vec![b'a' + i]).collect();
        let t = forks(
            labels
                .iter()
                .map(|l| labeled(l.clone(), leaf(l.clone())))
                .collect(),
        );
        for l in &labels {
            assert_eq!(
                t.lookup_path([l.as_slice()]),
                LookupResult::Found(l.as_slice())
            );
        }
        assert_eq!(t.lookup_path([b"0".as_slice()]), LookupResult::Absent);
        assert_eq!(t.lookup_path([b"ab".as_slice()]), LookupResult::Absent);
        assert_eq!(t.lookup_path([b"z".as_slice()]), LookupResult::Absent);
    }

    #[test]
    fn forks_edge_cases() {
        assert_eq!(forks(vec![]), empty());
        let one = labeled(b"x".to_vec(), leaf(b"y".to_vec()));
        assert_eq!(forks(vec![one.clone()]), one);
    }
}
