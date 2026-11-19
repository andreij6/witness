use crate::cert::{self, ROOT_LABEL, exact, forks, wildcard};
use crate::critbit::{Hash, Key};
use crate::html;
use crate::http::{self, Csp, Resp};
use crate::pins::Pins;
use crate::store::{Page, PageKind, Store};
use ic_certification::{HashTree, empty, labeled, pruned};
use ic_http_certification::utils::{add_v2_certificate_header, more_specific_wildcards_for};
use ic_http_certification::{HttpCertificationPath, HttpResponse};
use ic_stable_structures::Memory;
use std::borrow::Cow;
use witness_canister_common::{Origins, Removal};

pub const WELL_KNOWN: &[u8] = b".well-known";
pub const IC_DOMAINS: &[u8] = b"ic-domains";
pub const RECORDS: &[u8] = b"p";
pub const CASES: &[u8] = b"v";

pub struct Site {
    pub origins: Origins,
    pub csp: Csp,
    gone: [String; 3],
    domains: HttpResponse<'static>,
    not_found: HttpResponse<'static>,
    case: HttpResponse<'static>,
    domains_hash: Hash,
    not_found_hash: Hash,
    case_hash: Hash,
}

const fn gone_ix(why: Removal) -> usize {
    match why {
        Removal::Erased => 0,
        Removal::Takedown => 1,
        Removal::Revoked => 2,
    }
}

impl Site {
    pub fn new(origins: Origins, pins: &Pins) -> Self {
        let csp = Csp::new(&origins);
        let domains = http::response(Resp::Domains, html::ic_domains(&origins).into_bytes(), &csp);
        let not_found =
            http::response(Resp::NotFound, html::not_found(&origins).into_bytes(), &csp);
        let case = http::response(
            Resp::Case,
            html::case_shell(&origins, pins).into_bytes(),
            &csp,
        );
        let gone = [Removal::Erased, Removal::Takedown, Removal::Revoked]
            .map(|why| html::gone(why, &origins));
        Self {
            domains_hash: http::hash(&domains),
            not_found_hash: http::hash(&not_found),
            case_hash: http::hash(&case),
            origins,
            csp,
            gone,
            domains,
            not_found,
            case,
        }
    }

    pub fn gone(&self, why: Removal) -> &str {
        &self.gone[gone_ix(why)]
    }

    pub fn response<'a>(&self, kind: Resp, body: impl Into<Cow<'a, [u8]>>) -> HttpResponse<'a> {
        http::response(kind, body, &self.csp)
    }

    pub fn page_hash(&self, kind: PageKind, body: &[u8]) -> Hash {
        let body = match kind {
            PageKind::Gone(r) => self.gone(r).as_bytes(),
            _ => body,
        };
        http::hash(&self.response(kind.into(), body))
    }

    pub fn tree(&self, records: HashTree) -> HashTree {
        labeled(
            ROOT_LABEL,
            forks(vec![
                labeled(WELL_KNOWN, labeled(IC_DOMAINS, exact(&self.domains_hash))),
                wildcard(&self.not_found_hash),
                labeled(RECORDS, records),
                labeled(CASES, wildcard(&self.case_hash)),
            ]),
        )
    }

    pub fn certified_root(&self, records_root: Option<Hash>) -> Hash {
        self.tree(records_root.map_or_else(empty, pruned)).digest()
    }
}

pub fn tree_path(path: &str) -> Vec<Vec<u8>> {
    HttpCertificationPath::exact(path)
        .to_expr_path()
        .into_iter()
        .skip(1)
        .map(String::into_bytes)
        .collect()
}

enum Route {
    Record(Page),
    Domains,
    Case,
    NotFound,
}

fn route<M: Memory>(store: &Store<M>, req: &[Vec<u8>]) -> Route {
    match req {
        [p, id, t] if p == RECORDS && t == cert::EXACT => <Key>::try_from(id.as_slice())
            .ok()
            .and_then(|k| store.get(&k))
            .map_or(Route::NotFound, Route::Record),
        [w, d, t] if w == WELL_KNOWN && d == IC_DOMAINS && t == cert::EXACT => Route::Domains,
        [v, ..] if v == CASES => Route::Case,
        _ => Route::NotFound,
    }
}

pub fn serve<M: Memory>(
    s: &Site,
    store: &Store<M>,
    path: &str,
    certificate: &[u8],
) -> HttpResponse<'static> {
    let req = tree_path(path);
    let (mut resp, entry) = match route(store, &req) {
        Route::Record(page) => {
            let body = match page.kind {
                PageKind::Gone(r) => Cow::Owned(s.gone(r).as_bytes().to_vec()),
                _ => Cow::Owned(page.body),
            };
            (
                s.response(page.kind.into(), body),
                HttpCertificationPath::exact(path),
            )
        }
        Route::Domains => (s.domains.clone(), HttpCertificationPath::exact(path)),
        Route::Case => (s.case.clone(), HttpCertificationPath::wildcard("/v")),
        Route::NotFound => (s.not_found.clone(), HttpCertificationPath::wildcard("")),
    };
    let expr_path = entry.to_expr_path();
    let responding: Vec<Vec<u8>> = expr_path
        .iter()
        .skip(1)
        .map(|s| s.as_bytes().to_vec())
        .collect();
    let mut proofs = vec![req.clone(), responding.clone()];
    if responding.last().is_some_and(|t| t == cert::WILDCARD) {
        proofs.extend(more_specific_wildcards_for(&req, &responding));
    }
    let labels: Vec<&[u8]> = proofs
        .iter()
        .filter(|p| p.len() >= 2 && p[0] == RECORDS)
        .map(|p| p[1].as_slice())
        .collect();
    let witness = s.tree(store.witness(&labels));
    add_v2_certificate_header(certificate, &mut resp, &witness, &expr_path);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use ic_certification::SubtreeLookupResult;
    use ic_http_certification::{
        HttpCertification, HttpCertificationTree, HttpCertificationTreeEntry,
    };
    use ic_stable_structures::VectorMemory;
    use std::sync::LazyLock;

    type S = Store<VectorMemory>;

    static SITE: LazyLock<Site> = LazyLock::new(|| Site::new(Origins::default(), &Pins::default()));

    fn store() -> S {
        Store::init(
            VectorMemory::default(),
            VectorMemory::default(),
            VectorMemory::default(),
        )
    }

    fn put(s: &mut S, id: &str, kind: PageKind, body: &str) {
        let resp = match kind {
            PageKind::Gone(r) => SITE.response(kind.into(), SITE.gone(r).as_bytes()),
            _ => SITE.response(kind.into(), body.as_bytes().to_vec()),
        };
        let page = Page {
            kind,
            resp_hash: http::hash(&resp),
            body: if kind.is_gone() {
                Vec::new()
            } else {
                body.as_bytes().to_vec()
            },
        };
        s.put(id.as_bytes().try_into().unwrap(), page);
    }

    fn seeded() -> S {
        let mut s = store();
        for (i, id) in [
            "0000000000000000",
            "5555555555555555",
            "AAAAAAAAAAAAAAAA",
            "ZZZZZZZZZZZZZZZZ",
            "MMMMMMMMMMMMMMMM",
        ]
        .iter()
        .enumerate()
        {
            put(&mut s, id, PageKind::Shell, &format!("shell {i}"));
        }
        put(
            &mut s,
            "GGGGGGGGGGGGGGGG",
            PageKind::Gone(Removal::Takedown),
            "",
        );
        s
    }

    struct Parsed {
        tree: HashTree,
        expr_path: Vec<String>,
        certificate: Vec<u8>,
    }

    fn field<'a>(v: &'a str, name: &str) -> &'a str {
        let start = v.find(&format!("{name}=:")).unwrap() + name.len() + 2;
        let end = start + v[start..].find(':').unwrap();
        &v[start..end]
    }

    fn parse(resp: &HttpResponse) -> Parsed {
        let v = &resp
            .headers()
            .iter()
            .find(|(k, _)| k == "IC-Certificate")
            .unwrap()
            .1;
        assert!(v.ends_with("version=2"));
        let dec = |n| STANDARD.decode(field(v, n)).unwrap();
        Parsed {
            tree: serde_cbor::from_slice(&dec("tree")).unwrap(),
            expr_path: serde_cbor::from_slice(&dec("expr_path")).unwrap(),
            certificate: dec("certificate"),
        }
    }

    fn verify(s: &S, path: &str, resp: &HttpResponse) -> Parsed {
        let p = parse(resp);
        assert_eq!(
            p.tree.digest(),
            SITE.certified_root(s.root_hash()),
            "root for {path}"
        );
        let mut lookup: Vec<Vec<u8>> = p.expr_path.iter().map(|x| x.as_bytes().to_vec()).collect();
        let SubtreeLookupResult::Found(entry) = p.tree.lookup_subtree(&lookup) else {
            panic!("expr path not found for {path}");
        };
        let uncertified = HttpResponse::builder()
            .with_status_code(resp.status_code())
            .with_headers(
                resp.headers()
                    .iter()
                    .filter(|(k, _)| k != "IC-Certificate")
                    .cloned()
                    .collect(),
            )
            .with_body(resp.body().to_vec())
            .build();
        let rh = http::hash(&uncertified);
        lookup.extend([cert::CEL.hash.to_vec(), Vec::new(), rh.to_vec()]);
        assert!(
            matches!(
                p.tree.lookup_subtree(&lookup),
                SubtreeLookupResult::Found(_)
            ),
            "response hash not certified for {path}"
        );
        assert!(entry.digest() != [0u8; 32]);
        let req = tree_path(path);
        let responding: Vec<Vec<u8>> = p.expr_path[1..]
            .iter()
            .map(|x| x.as_bytes().to_vec())
            .collect();
        if responding != req {
            let mut exact = vec![ROOT_LABEL.to_vec()];
            exact.extend(req.iter().cloned());
            assert_eq!(
                p.tree.lookup_subtree(&exact),
                SubtreeLookupResult::Absent,
                "exact {path}"
            );
            for w in more_specific_wildcards_for(&req, &responding) {
                let mut full = vec![ROOT_LABEL.to_vec()];
                full.extend(w);
                assert_eq!(
                    p.tree.lookup_subtree(&full),
                    SubtreeLookupResult::Absent,
                    "{path} {full:?}"
                );
            }
        }
        p
    }

    #[test]
    fn found_shell_is_certified_exactly() {
        let s = seeded();
        let r = serve(&SITE, &s, "/p/5555555555555555", b"cert");
        assert_eq!(r.status_code(), 200);
        assert_eq!(r.body(), b"shell 1");
        let p = verify(&s, "/p/5555555555555555", &r);
        assert_eq!(p.expr_path, ["http_expr", "p", "5555555555555555", "<$>"]);
        assert_eq!(p.certificate, b"cert");
    }

    #[test]
    fn tombstone_served_with_410() {
        let s = seeded();
        let r = serve(&SITE, &s, "/p/GGGGGGGGGGGGGGGG", b"c");
        assert_eq!(r.status_code(), 410);
        assert_eq!(r.body(), SITE.gone(Removal::Takedown).as_bytes());
        verify(&s, "/p/GGGGGGGGGGGGGGGG", &r);
    }

    #[test]
    fn missing_paths_fall_back_to_certified_404() {
        let s = seeded();
        for path in [
            "/p/1111111111111111",
            "/p/5555555555555556",
            "/p/5555555555555555/x",
            "/p/5555555555555555/",
            "/p/<*>",
            "/p/",
            "/p",
            "/",
            "",
            "/q/a/b/c",
            "/p/0",
            "/p/zzzzzzzzzzzzzzzzzzzzzzzz",
            "/.well-known",
            "/.well-known/ic-domains/x",
        ] {
            let r = serve(&SITE, &s, path, b"c");
            assert_eq!(r.status_code(), 404, "{path}");
            let p = verify(&s, path, &r);
            assert_eq!(p.expr_path, ["http_expr", "<*>"], "{path}");
        }
    }

    #[test]
    fn missing_record_on_empty_store_is_404() {
        let s = store();
        let r = serve(&SITE, &s, "/p/AAAAAAAAAAAAAAAA", b"c");
        assert_eq!(r.status_code(), 404);
        verify(&s, "/p/AAAAAAAAAAAAAAAA", &r);
    }

    #[test]
    fn case_wildcard_hides_case_ids() {
        let s = seeded();
        let a = serve(&SITE, &s, "/v/ABCDEF", b"c");
        let b = serve(&SITE, &s, "/v/ZYXW/deeper", b"c");
        assert_eq!(a.status_code(), 200);
        assert_eq!(a.body(), b.body());
        assert_eq!(
            a.body(),
            html::case_shell(&Origins::default(), &Pins::default()).as_bytes()
        );
        for (path, r) in [
            ("/v/ABCDEF", &a),
            ("/v/ZYXW/deeper", &b),
            ("/v", &serve(&SITE, &s, "/v", b"c")),
        ] {
            let p = verify(&s, path, r);
            assert_eq!(p.expr_path, ["http_expr", "v", "<*>"]);
        }
    }

    #[test]
    fn pinned_case_shell_is_certified_and_moves_only_the_case_hash() {
        let s = seeded();
        let pinned = Site::new(Origins::default(), &crate::pins::tests::sample());
        let r = serve(&pinned, &s, "/v/ABCDEF", b"c");
        assert_eq!(
            r.body(),
            html::case_shell(&Origins::default(), &crate::pins::tests::sample()).as_bytes()
        );
        assert!(String::from_utf8_lossy(r.body()).contains("type=\"importmap\""));
        assert_ne!(pinned.case_hash, SITE.case_hash);
        assert_eq!(pinned.not_found_hash, SITE.not_found_hash);
        assert_eq!(pinned.domains_hash, SITE.domains_hash);
        assert_eq!(pinned.gone, SITE.gone);
        assert_ne!(
            pinned.certified_root(s.root_hash()),
            SITE.certified_root(s.root_hash())
        );
        assert_eq!(
            pinned.page_hash(PageKind::Gone(Removal::Erased), &[]),
            SITE.page_hash(PageKind::Gone(Removal::Erased), &[])
        );
        assert!(
            r.headers()
                .iter()
                .any(|(k, v)| k == "content-security-policy" && *v == pinned.csp.case)
        );
    }

    #[test]
    fn ic_domains_is_exact() {
        let s = seeded();
        let r = serve(&SITE, &s, "/.well-known/ic-domains", b"c");
        assert_eq!(r.body(), html::ic_domains(&Origins::default()).as_bytes());
        let p = verify(&s, "/.well-known/ic-domains", &r);
        assert_eq!(
            p.expr_path,
            ["http_expr", ".well-known", "ic-domains", "<$>"]
        );
    }

    #[test]
    fn root_tracks_store_and_matches_reference_for_single_record() {
        let mut s = store();
        let empty_root = SITE.certified_root(None);
        put(&mut s, "AAAAAAAAAAAAAAAA", PageKind::Shell, "x");
        let one = SITE.certified_root(s.root_hash());
        assert_ne!(one, empty_root);
        let mut reference = HttpCertificationTree::default();
        let st = &*SITE;
        let add = |t: &mut HttpCertificationTree,
                   path: HttpCertificationPath<'static>,
                   r: &HttpResponse| {
            let c = HttpCertification::response_only(&cert::CEL.def, r, None).unwrap();
            t.insert(&HttpCertificationTreeEntry::new(path, c));
        };
        add(
            &mut reference,
            HttpCertificationPath::exact("/.well-known/ic-domains"),
            &st.domains,
        );
        add(
            &mut reference,
            HttpCertificationPath::wildcard(""),
            &st.not_found,
        );
        add(
            &mut reference,
            HttpCertificationPath::wildcard("/v"),
            &st.case,
        );
        add(
            &mut reference,
            HttpCertificationPath::exact("/p/AAAAAAAAAAAAAAAA"),
            &SITE.response(Resp::Shell, b"x".to_vec()),
        );
        let ours = serve(&SITE, &s, "/p/AAAAAAAAAAAAAAAA", b"c");
        let ref_w = reference
            .witness(
                &HttpCertificationTreeEntry::new(
                    HttpCertificationPath::exact("/p/AAAAAAAAAAAAAAAA"),
                    HttpCertification::response_only(
                        &cert::CEL.def,
                        &SITE.response(Resp::Shell, b"x".to_vec()),
                        None,
                    )
                    .unwrap(),
                ),
                "/p/AAAAAAAAAAAAAAAA",
            )
            .unwrap();
        let path = ["http_expr", "p", "AAAAAAAAAAAAAAAA", "<$>"];
        let SubtreeLookupResult::Found(a) = parse(&ours).tree.lookup_subtree(path) else {
            panic!()
        };
        let SubtreeLookupResult::Found(b) = ref_w.lookup_subtree(path) else {
            panic!()
        };
        assert_eq!(a.digest(), b.digest());
        for p in [
            vec!["http_expr", "<*>"],
            vec!["http_expr", "v", "<*>"],
            vec!["http_expr", ".well-known", "ic-domains"],
        ] {
            let SubtreeLookupResult::Found(a) = SITE.tree(empty()).lookup_subtree(&p) else {
                panic!()
            };
            let SubtreeLookupResult::Found(b) = reference.as_hash_tree().lookup_subtree(&p) else {
                panic!()
            };
            assert_eq!(a.digest(), b.digest(), "{p:?}");
        }
    }

    #[test]
    fn serve_is_deterministic() {
        let s = seeded();
        for path in ["/p/AAAAAAAAAAAAAAAA", "/p/nope", "/v/x"] {
            let a = serve(&SITE, &s, path, b"c");
            let b = serve(&SITE, &s, path, b"c");
            assert_eq!(a.headers(), b.headers());
            assert_eq!(a.body(), b.body());
        }
    }

    #[test]
    fn many_records_keep_witnesses_small() {
        let mut s = store();
        const A: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let mut x = 1u64;
        for _ in 0..3000 {
            let id: String = (0..16)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    A[(x % 32) as usize] as char
                })
                .collect();
            put(&mut s, &id, PageKind::Shell, &id);
        }
        let r = serve(&SITE, &s, "/p/ZZZZZZZZZZZZZZZZ", b"c");
        verify(&s, "/p/ZZZZZZZZZZZZZZZZ", &r);
        let header = &r
            .headers()
            .iter()
            .find(|(k, _)| k == "IC-Certificate")
            .unwrap()
            .1;
        assert!(header.len() < 8 * 1024, "header {} bytes", header.len());
    }
}
