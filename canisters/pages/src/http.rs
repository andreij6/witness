use crate::cert::CEL;
use crate::critbit::Hash;
use crate::html::CASE_NONCE;
use crate::store::PageKind;
use ic_http_certification::{
    CERTIFICATE_EXPRESSION_HEADER_NAME, HeaderField, HttpResponse, StatusCode, response_hash,
};
use std::borrow::Cow;
use witness_canister_common::{Origins, Removal};

pub const CSP_STATIC: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csp {
    pub proof: String,
    pub case: String,
}

impl Csp {
    pub fn new(o: &Origins) -> Self {
        let connect = format!("connect-src {} {} {}", o.app, o.api, o.canister);
        let img = format!("img-src 'self' {} blob: data:", o.canister);
        let tail = format!(
            "style-src 'self' 'unsafe-inline'; font-src {}; base-uri 'none'; frame-ancestors 'none'",
            o.app
        );
        Self {
            proof: format!(
                "default-src 'none'; script-src 'self' {} 'wasm-unsafe-eval'; {connect}; {img}; {tail}",
                o.app
            ),
            case: format!(
                "default-src 'none'; script-src 'nonce-{}' 'strict-dynamic' 'wasm-unsafe-eval'; {connect} blob:; {img}; {tail}",
                CASE_NONCE.as_str()
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resp {
    Shell,
    DevShell,
    Gone(Removal),
    NotFound,
    Case,
    Domains,
}

impl From<PageKind> for Resp {
    fn from(k: PageKind) -> Self {
        match k {
            PageKind::Shell => Self::Shell,
            PageKind::DevShell => Self::DevShell,
            PageKind::Gone(r) => Self::Gone(r),
        }
    }
}

impl Resp {
    pub fn status(self) -> StatusCode {
        match self {
            Self::Shell | Self::DevShell | Self::Case | Self::Domains => StatusCode::OK,
            Self::Gone(_) => StatusCode::GONE,
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn noindex(self) -> bool {
        !matches!(self, Self::Shell | Self::Domains)
    }

    fn cache(self) -> &'static str {
        match self {
            Self::NotFound | Self::Case => "no-cache",
            Self::Domains => "public, max-age=3600",
            _ => "public, max-age=300",
        }
    }

    pub fn headers(self, csp: &Csp) -> Vec<HeaderField> {
        let mut h: Vec<HeaderField> = Vec::with_capacity(8);
        let mut add = |k: &str, v: String| h.push((k.to_string(), v));
        add(CERTIFICATE_EXPRESSION_HEADER_NAME, CEL.text.clone());
        add(
            "content-type",
            match self {
                Self::Domains => "text/plain; charset=utf-8",
                _ => "text/html; charset=utf-8",
            }
            .to_string(),
        );
        add("x-content-type-options", "nosniff".to_string());
        add("cache-control", self.cache().to_string());
        match self {
            Self::Shell | Self::DevShell => {
                add("content-security-policy", csp.proof.clone());
                add(
                    "referrer-policy",
                    "strict-origin-when-cross-origin".to_string(),
                );
            }
            Self::Case => {
                add("content-security-policy", csp.case.clone());
                add("referrer-policy", "no-referrer".to_string());
            }
            Self::Gone(_) | Self::NotFound => {
                add("content-security-policy", CSP_STATIC.to_string());
                add("referrer-policy", "no-referrer".to_string());
            }
            Self::Domains => {}
        }
        if self.noindex() {
            add("x-robots-tag", "noindex, nofollow".to_string());
        }
        h
    }
}

pub fn response<'a>(kind: Resp, body: impl Into<Cow<'a, [u8]>>, csp: &Csp) -> HttpResponse<'a> {
    HttpResponse::builder()
        .with_status_code(kind.status())
        .with_headers(kind.headers(csp))
        .with_body(body)
        .build()
}

pub fn hash(resp: &HttpResponse) -> Hash {
    response_hash(resp, &CEL.def.response, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_http_certification::HttpCertification;
    use std::sync::LazyLock;

    static PROD: LazyLock<Csp> = LazyLock::new(|| Csp::new(&Origins::default()));

    fn local() -> Origins {
        Origins {
            site: "http://pages.localhost:8090".into(),
            case: "http://pages.localhost:8090".into(),
            app: "http://web.localhost:8090".into(),
            api: "http://localhost:8090".into(),
            canister: "http://*.localhost:8090".into(),
        }
    }

    const ALL: [Resp; 8] = [
        Resp::Shell,
        Resp::DevShell,
        Resp::Gone(Removal::Erased),
        Resp::Gone(Removal::Takedown),
        Resp::Gone(Removal::Revoked),
        Resp::NotFound,
        Resp::Case,
        Resp::Domains,
    ];

    fn header<'a>(h: &'a [HeaderField], name: &str) -> Option<&'a str> {
        h.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    #[test]
    fn every_response_is_certifiable() {
        for k in ALL {
            let r = response(k, b"x".as_slice(), &PROD);
            assert!(
                HttpCertification::response_only(&CEL.def, &r, None).is_ok(),
                "{k:?}"
            );
            assert_eq!(r.status_code(), k.status());
        }
    }

    #[test]
    fn noindex_rules() {
        for k in ALL {
            let h = k.headers(&PROD);
            let expect = !matches!(k, Resp::Shell | Resp::Domains);
            assert_eq!(header(&h, "x-robots-tag").is_some(), expect, "{k:?}");
        }
    }

    #[test]
    fn csp_per_origin() {
        let shell = Resp::Shell.headers(&PROD);
        let proof = header(&shell, "content-security-policy").unwrap();
        assert!(proof.starts_with(
            "default-src 'none'; script-src 'self' https://app.witness.app 'wasm-unsafe-eval'"
        ));
        assert!(
            proof.contains(
                "connect-src https://app.witness.app https://icp-api.io https://*.icp0.io;"
            )
        );
        let case = Resp::Case.headers(&PROD);
        let c = header(&case, "content-security-policy").unwrap();
        assert!(c.contains(&format!(
            "script-src 'nonce-{}' 'strict-dynamic' 'wasm-unsafe-eval'",
            CASE_NONCE.as_str()
        )));
        assert!(c.contains("https://*.icp0.io blob:;"));
        assert!(!c.contains("script-src 'self'"));
        assert_eq!(header(&case, "referrer-policy"), Some("no-referrer"));
        assert!(header(&Resp::Domains.headers(&PROD), "content-security-policy").is_none());
        assert_eq!(
            header(&Resp::Domains.headers(&PROD), "content-type"),
            Some("text/plain; charset=utf-8")
        );
    }

    #[test]
    fn csp_lists_exactly_the_configured_origins() {
        let o = local();
        let csp = Csp::new(&o);
        assert_eq!(
            csp.proof,
            "default-src 'none'; script-src 'self' http://web.localhost:8090 'wasm-unsafe-eval'; connect-src http://web.localhost:8090 http://localhost:8090 http://*.localhost:8090; img-src 'self' http://*.localhost:8090 blob: data:; style-src 'self' 'unsafe-inline'; font-src http://web.localhost:8090; base-uri 'none'; frame-ancestors 'none'"
        );
        for policy in [&csp.proof, &csp.case] {
            assert!(!policy.contains("witness.app"));
            assert!(!policy.contains("icp0.io"));
            assert!(!policy.contains("icp-api.io"));
        }
        assert!(csp.case.contains("connect-src http://web.localhost:8090 http://localhost:8090 http://*.localhost:8090 blob:;"));
        let h = Resp::Shell.headers(&csp);
        assert_eq!(
            header(&h, "content-security-policy"),
            Some(csp.proof.as_str())
        );
        let h = Resp::NotFound.headers(&csp);
        assert_eq!(header(&h, "content-security-policy"), Some(CSP_STATIC));
    }

    #[test]
    fn csp_changes_the_certified_hash() {
        let local = Csp::new(&local());
        assert_ne!(
            hash(&response(Resp::Shell, b"a".as_slice(), &PROD)),
            hash(&response(Resp::Shell, b"a".as_slice(), &local))
        );
        assert_eq!(
            hash(&response(Resp::NotFound, b"a".as_slice(), &PROD)),
            hash(&response(Resp::NotFound, b"a".as_slice(), &local))
        );
    }

    #[test]
    fn hash_depends_on_kind_and_body() {
        let a = hash(&response(Resp::Shell, b"a".as_slice(), &PROD));
        assert_eq!(a, hash(&response(Resp::Shell, b"a".to_vec(), &PROD)));
        assert_ne!(a, hash(&response(Resp::Shell, b"b".as_slice(), &PROD)));
        assert_ne!(a, hash(&response(Resp::DevShell, b"a".as_slice(), &PROD)));
        assert_ne!(
            hash(&response(
                Resp::Gone(Removal::Erased),
                b"a".as_slice(),
                &PROD
            )),
            hash(&response(Resp::NotFound, b"a".as_slice(), &PROD))
        );
    }

    #[test]
    fn page_kinds_map_to_responses() {
        assert_eq!(Resp::from(PageKind::Shell), Resp::Shell);
        assert_eq!(Resp::from(PageKind::DevShell), Resp::DevShell);
        assert_eq!(
            Resp::from(PageKind::Gone(Removal::Revoked)),
            Resp::Gone(Removal::Revoked)
        );
    }

    #[test]
    fn the_pinned_case_shell_is_revalidated_on_every_load() {
        for k in ALL {
            let h = k.headers(&PROD);
            let expect = match k {
                Resp::Case | Resp::NotFound => "no-cache",
                Resp::Domains => "public, max-age=3600",
                _ => "public, max-age=300",
            };
            assert_eq!(header(&h, "cache-control"), Some(expect), "{k:?}");
        }
    }

    #[test]
    fn statuses() {
        assert_eq!(Resp::Gone(Removal::Erased).status(), StatusCode::GONE);
        assert_eq!(Resp::NotFound.status(), StatusCode::NOT_FOUND);
        assert_eq!(Resp::Case.status(), StatusCode::OK);
    }
}
