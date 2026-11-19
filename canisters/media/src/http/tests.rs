use super::*;
use candid::{Decode, Encode, Principal};
use ic_certification::{empty, leaf};
use ic_http_certification::{DefaultResponseOnlyCelExpression, HttpCertification};
use std::sync::LazyLock;
use witness_format::RecordId;
use witness_format::crypto::sha256;

static PROD: LazyLock<Origins> = LazyLock::new(Origins::default);

fn local() -> Origins {
    Origins {
        site: "http://pages-id.localhost:8090".into(),
        case: "http://pages-id.localhost:8090".into(),
        app: "http://web-id.localhost:8090".into(),
        api: "http://localhost:8090".into(),
        canister: "http://*.localhost:8090".into(),
    }
}

fn path() -> BlobPath {
    BlobPath::record(Kind::Preview, &RecordId::from_bytes([3; 10]))
}

fn cel_def() -> DefaultResponseOnlyCelExpression<'static> {
    DefaultCelBuilder::response_only_certification()
        .with_response_certification(DefaultResponseCertification::certified_response_headers(
            CERTIFIED.as_slice(),
        ))
        .build()
}

fn served<'a>(
    p: &'a BlobPath,
    body: Vec<u8>,
    size: u64,
    w: &'a HashTree,
    origin: Option<&'a str>,
) -> Served<'a> {
    Served {
        path: p,
        blob: 7,
        size,
        body,
        witness: w,
        certificate: b"cert",
        origin,
        origins: &PROD,
        local: false,
        callback: Some(StreamingCallback::new(
            Principal::anonymous(),
            "http_request_streaming_callback".into(),
        )),
    }
}

fn lib_hash(r: &HttpResponse) -> Hash {
    let lib = LibResponse::builder()
        .with_status_code(StatusCode::OK)
        .with_headers(r.headers.clone())
        .with_body(r.body.clone())
        .build();
    response_hash(
        &lib,
        &DefaultResponseCertification::certified_response_headers(CERTIFIED.as_slice()),
        None,
    )
}

#[test]
fn cel_expression_is_stable_and_hashed() {
    assert_eq!(cel_expr(), cel_def().to_string());
    assert!(cel_expr().contains("content-type"));
    assert!(cel_expr().contains("cache-control"));
    assert_eq!(expr_hash(), sha256(cel_expr().as_bytes()));
    assert_eq!(expr_hash(), expr_hash());
}

#[test]
fn served_response_hash_matches_library() {
    let p = path();
    let w = empty();
    let body = b"jpeg bytes".to_vec();
    let r = blob_response(served(
        &p,
        body.clone(),
        body.len() as u64,
        &w,
        Some("https://witness.app"),
    ));
    assert_eq!(
        lib_hash(&r),
        blob_response_hash(Kind::Preview, &sha256(&body))
    );
    let lib = LibResponse::builder()
        .with_status_code(StatusCode::OK)
        .with_headers(r.headers.clone())
        .with_body(body)
        .build();
    assert!(HttpCertification::response_only(&cel_def(), &lib, None).is_ok());
}

#[test]
fn response_hash_depends_on_kind_and_body() {
    let a = blob_response_hash(Kind::Preview, &[1; 32]);
    assert_ne!(a, blob_response_hash(Kind::Preview, &[2; 32]));
    assert_ne!(a, blob_response_hash(Kind::Case, &[1; 32]));
    assert_eq!(a, blob_response_hash(Kind::Og, &[1; 32]));
    assert_eq!(a, blob_response_hash(Kind::Preview, &[1; 32]));
}

#[test]
fn origins() {
    for o in PROD.browser_origins() {
        assert!(allowed_origin(o, &PROD, false));
        assert!(allowed_origin(o, &PROD, true));
    }
    assert!(!allowed_origin("http://localhost", &PROD, false));
    assert!(allowed_origin("http://localhost", &PROD, true));
    assert!(allowed_origin("http://localhost:8080", &PROD, true));
    for bad in [
        "http://localhost:",
        "http://localhost:80a",
        "http://localhost:123456",
        "http://localhost.evil.com",
        "http://localhost:80/x",
        "https://witness.app.evil.com",
        "https://evil.witness.app",
        "http://witness.app",
        "",
        "null",
    ] {
        assert!(!allowed_origin(bad, &PROD, true), "{bad}");
    }
}

#[test]
fn configured_local_origins_are_allowed_only_in_local_dev() {
    let o = local();
    for origin in [
        "http://pages-id.localhost:8090",
        "http://web-id.localhost:8090",
    ] {
        assert!(allowed_origin(origin, &o, true), "{origin}");
        assert!(!allowed_origin(origin, &o, false), "{origin}");
        assert!(!allowed_origin(origin, &PROD, true), "{origin}");
    }
    for bad in [
        "http://localhost:8090",
        "http://other-id.localhost:8090",
        "http://web-id.localhost:8091",
        "https://web-id.localhost:8090",
        "http://web-id.localhost:8090/",
        "http://*.localhost:8090",
        "https://witness.app",
    ] {
        assert!(
            !allowed_origin(bad, &o, true) || bad == "http://localhost:8090",
            "{bad}"
        );
        assert!(!allowed_origin(bad, &o, false), "{bad}");
    }
}

#[test]
fn not_found_is_certifiable_with_the_shared_expression() {
    let r = not_found(&empty(), b"cert");
    assert_eq!(r.status_code, 404);
    assert_eq!(r.body, NOT_FOUND_BODY);
    let lib = LibResponse::builder()
        .with_status_code(StatusCode::NOT_FOUND)
        .with_headers(r.headers.clone())
        .with_body(r.body.clone())
        .build();
    assert!(HttpCertification::response_only(&cel_def(), &lib, None).is_ok());
    let lib_hash = response_hash(
        &lib,
        &DefaultResponseCertification::certified_response_headers(CERTIFIED.as_slice()),
        None,
    );
    assert_eq!(lib_hash, not_found_hash());
    assert_eq!(not_found_expr_path(), ["http_expr", "<*>"]);
    let cert = header(&r.headers, "ic-certificate").unwrap_or_default();
    assert!(cert.contains("version=2") && cert.contains("expr_path=:"));
    assert_eq!(header(&r.headers, "cache-control"), Some("no-cache"));
    assert_eq!(header(&r.headers, "access-control-allow-origin"), None);
    assert_ne!(
        not_found_hash(),
        blob_response_hash(Kind::Preview, &sha256(NOT_FOUND_BODY))
    );
}

#[test]
fn header_lookup_ignores_case() {
    let h = vec![("Origin".to_string(), "x".to_string())];
    assert_eq!(header(&h, "origin"), Some("x"));
    assert_eq!(header(&h, "ORIGIN"), Some("x"));
    assert_eq!(header(&h, "host"), None);
    assert_eq!(header(&[], "origin"), None);
}

#[test]
fn expr_path_layout() {
    let p = path();
    let e = expr_path(&p);
    assert_eq!(e.len(), 4);
    assert_eq!(e[0], "http_expr");
    assert_eq!(e[1], "p");
    assert_eq!(e[2].as_bytes(), p.label());
    assert_eq!(e[3], "<$>");
}

#[test]
fn small_blob_has_no_streaming_and_cors_when_allowed() {
    let p = path();
    let w = leaf(b"x".to_vec());
    let r = blob_response(served(
        &p,
        vec![1, 2, 3],
        3,
        &w,
        Some("https://case.witness.app"),
    ));
    assert_eq!(r.status_code, 200);
    assert!(r.streaming_strategy.is_none());
    assert_eq!(
        header(&r.headers, "access-control-allow-origin"),
        Some("https://case.witness.app")
    );
    assert_eq!(header(&r.headers, "vary"), Some("Origin"));
    assert_eq!(header(&r.headers, "cache-control"), Some(CACHE_CONTROL));
    assert_eq!(header(&r.headers, "content-type"), Some("image/jpeg"));
    let cert = header(&r.headers, "ic-certificate").unwrap_or_default();
    assert!(cert.contains("version=2") && cert.contains("expr_path=:"));
}

#[test]
fn disallowed_origin_gets_no_cors() {
    let p = path();
    let w = empty();
    for origin in [Some("https://evil.example"), None] {
        let r = blob_response(served(&p, vec![1], 1, &w, origin));
        assert_eq!(header(&r.headers, "access-control-allow-origin"), None);
        assert_eq!(header(&r.headers, "vary"), None);
    }
}

#[test]
fn large_blob_streams_from_first_chunk() {
    let p = path();
    let w = empty();
    let size = STREAM_CHUNK * 2 + 5;
    let r = blob_response(served(
        &p,
        vec![0; chunk_len(size, 0) as usize],
        size,
        &w,
        None,
    ));
    match r.streaming_strategy {
        Some(StreamingStrategy::Callback { token, callback }) => {
            assert_eq!(
                token,
                StreamingToken {
                    blob: 7,
                    offset: STREAM_CHUNK
                }
            );
            assert_eq!(callback.0.method, "http_request_streaming_callback");
        }
        None => panic!("expected streaming"),
    }
    let r = blob_response(Served {
        callback: None,
        ..served(&p, vec![0; 3], size, &w, None)
    });
    assert!(r.streaming_strategy.is_none());
}

#[test]
fn chunk_helpers() {
    assert_eq!(chunk_len(0, 0), 0);
    assert_eq!(chunk_len(10, 0), 10);
    assert_eq!(chunk_len(STREAM_CHUNK, 0), STREAM_CHUNK);
    assert_eq!(chunk_len(STREAM_CHUNK + 1, 0), STREAM_CHUNK);
    assert_eq!(chunk_len(STREAM_CHUNK + 1, STREAM_CHUNK), 1);
    assert_eq!(chunk_len(5, 9), 0);
    assert_eq!(chunk_len(u64::MAX, u64::MAX - 1), 1);
    assert_eq!(chunk_token(1, 10, 0, 10), None);
    assert_eq!(
        chunk_token(1, 10, 0, 4),
        Some(StreamingToken { blob: 1, offset: 4 })
    );
    assert_eq!(chunk_token(1, 10, 6, 4), None);
}

#[test]
fn plain_response() {
    let r = plain(404, "not found");
    assert_eq!(r.status_code, 404);
    assert_eq!(r.body, b"not found");
    assert!(r.streaming_strategy.is_none());
}

#[test]
fn candid_round_trip() {
    let p = path();
    let w = empty();
    let r = blob_response(served(&p, vec![9; 4], STREAM_CHUNK * 3, &w, None));
    let bytes = Encode!(&r).unwrap_or_default();
    assert_eq!(Decode!(&bytes, HttpResponse).ok(), Some(r));
    let c = StreamingCallbackHttpResponse {
        body: vec![1],
        token: Some(StreamingToken { blob: 1, offset: 2 }),
    };
    let bytes = Encode!(&c).unwrap_or_default();
    assert_eq!(Decode!(&bytes, StreamingCallbackHttpResponse).ok(), Some(c));
}
