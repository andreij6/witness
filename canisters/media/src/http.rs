use crate::certtree::{EXACT, EXPR_PREFIX, Hash, WILDCARD};
use crate::paths::{BlobPath, Kind};
use candid::{CandidType, Deserialize};
use ic_certification::HashTree;
use ic_http_certification::utils::{build_v2_certificate_header_value, cbor_encode_to_base64};
use ic_http_certification::{
    CERTIFICATE_EXPRESSION_HEADER_NAME, CERTIFICATE_HEADER_NAME, DefaultCelBuilder,
    DefaultResponseCertification, HttpResponse as LibResponse, StatusCode, response_hash,
};
use std::sync::OnceLock;
use witness_canister_common::Origins;

pub const STREAM_CHUNK: u64 = 2 * 1024 * 1024;
pub const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
pub const LOCAL_ORIGIN: &str = "http://localhost";
pub const NOT_FOUND_BODY: &[u8] = b"not found";
const NOT_FOUND_CACHE: &str = "no-cache";
const CERTIFIED: [&str; 2] = ["content-type", "cache-control"];

pub type HeaderField = (String, String);

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StreamingToken {
    pub blob: u64,
    pub offset: u64,
}

candid::define_function!(pub StreamingCallback : (StreamingToken) -> (StreamingCallbackHttpResponse) query);

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum StreamingStrategy {
    Callback {
        callback: StreamingCallback,
        token: StreamingToken,
    },
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StreamingCallbackHttpResponse {
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
    pub token: Option<StreamingToken>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
    pub streaming_strategy: Option<StreamingStrategy>,
    pub upgrade: Option<bool>,
}

pub fn cel_expr() -> &'static str {
    static EXPR: OnceLock<String> = OnceLock::new();
    EXPR.get_or_init(|| {
        DefaultCelBuilder::response_only_certification()
            .with_response_certification(DefaultResponseCertification::certified_response_headers(
                CERTIFIED.as_slice(),
            ))
            .build()
            .to_string()
    })
}

pub fn expr_hash() -> Hash {
    static HASH: OnceLock<Hash> = OnceLock::new();
    *HASH.get_or_init(|| witness_format::crypto::sha256(cel_expr().as_bytes()))
}

fn certified_headers(content_type: &str, cache: &str) -> Vec<HeaderField> {
    vec![
        ("content-type".into(), content_type.into()),
        ("cache-control".into(), cache.into()),
        (
            CERTIFICATE_EXPRESSION_HEADER_NAME.into(),
            cel_expr().to_owned(),
        ),
    ]
}

pub fn blob_response_hash(kind: Kind, body_sha256: &Hash) -> Hash {
    let resp = LibResponse::builder()
        .with_status_code(StatusCode::OK)
        .with_headers(certified_headers(kind.content_type(), CACHE_CONTROL))
        .build();
    response_hash(
        &resp,
        &DefaultResponseCertification::certified_response_headers(CERTIFIED.as_slice()),
        Some(*body_sha256),
    )
}

fn not_found_headers() -> Vec<HeaderField> {
    certified_headers("text/plain; charset=utf-8", NOT_FOUND_CACHE)
}

pub fn not_found_hash() -> Hash {
    static HASH: OnceLock<Hash> = OnceLock::new();
    *HASH.get_or_init(|| {
        let resp = LibResponse::builder()
            .with_status_code(StatusCode::NOT_FOUND)
            .with_headers(not_found_headers())
            .with_body(NOT_FOUND_BODY)
            .build();
        response_hash(
            &resp,
            &DefaultResponseCertification::certified_response_headers(CERTIFIED.as_slice()),
            None,
        )
    })
}

pub fn not_found_expr_path() -> Vec<String> {
    [EXPR_PREFIX, WILDCARD]
        .iter()
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

pub fn not_found(witness: &HashTree, certificate: &[u8]) -> HttpResponse {
    let mut headers = not_found_headers();
    headers.push((
        CERTIFICATE_HEADER_NAME.into(),
        build_v2_certificate_header_value(
            certificate,
            witness,
            &cbor_encode_to_base64(&not_found_expr_path()),
        ),
    ));
    headers.push(("x-content-type-options".into(), "nosniff".into()));
    HttpResponse {
        status_code: 404,
        headers,
        body: NOT_FOUND_BODY.to_vec(),
        streaming_strategy: None,
        upgrade: None,
    }
}

pub fn allowed_origin(origin: &str, origins: &Origins, local: bool) -> bool {
    if origins
        .browser_origins()
        .iter()
        .any(|o| *o == origin && (local || o.starts_with("https://")))
    {
        return true;
    }
    local
        && origin.strip_prefix(LOCAL_ORIGIN).is_some_and(|rest| {
            rest.is_empty()
                || rest.strip_prefix(':').is_some_and(|p| {
                    !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit())
                })
        })
}

pub fn header<'a>(headers: &'a [HeaderField], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub fn expr_path(path: &BlobPath) -> Vec<String> {
    vec![
        String::from_utf8_lossy(EXPR_PREFIX).into_owned(),
        path.kind().segment().to_owned(),
        String::from_utf8_lossy(path.label()).into_owned(),
        String::from_utf8_lossy(EXACT).into_owned(),
    ]
}

pub struct Served<'a> {
    pub path: &'a BlobPath,
    pub blob: u64,
    pub size: u64,
    pub body: Vec<u8>,
    pub witness: &'a HashTree,
    pub certificate: &'a [u8],
    pub origin: Option<&'a str>,
    pub origins: &'a Origins,
    pub local: bool,
    pub callback: Option<StreamingCallback>,
}

pub fn blob_response(s: Served<'_>) -> HttpResponse {
    let mut headers = certified_headers(s.path.kind().content_type(), CACHE_CONTROL);
    headers.push((
        CERTIFICATE_HEADER_NAME.into(),
        build_v2_certificate_header_value(
            s.certificate,
            s.witness,
            &cbor_encode_to_base64(&expr_path(s.path)),
        ),
    ));
    headers.push(("x-content-type-options".into(), "nosniff".into()));
    if let Some(o) = s.origin.filter(|o| allowed_origin(o, s.origins, s.local)) {
        headers.push(("access-control-allow-origin".into(), o.to_owned()));
        headers.push(("vary".into(), "Origin".into()));
    }
    let sent = s.body.len() as u64;
    let streaming_strategy =
        s.callback
            .filter(|_| sent < s.size)
            .map(|callback| StreamingStrategy::Callback {
                callback,
                token: StreamingToken {
                    blob: s.blob,
                    offset: sent,
                },
            });
    HttpResponse {
        status_code: 200,
        headers,
        body: s.body,
        streaming_strategy,
        upgrade: None,
    }
}

pub fn chunk_len(size: u64, offset: u64) -> u64 {
    size.saturating_sub(offset).min(STREAM_CHUNK)
}

pub fn chunk_token(blob: u64, size: u64, offset: u64, len: u64) -> Option<StreamingToken> {
    (offset + len < size).then_some(StreamingToken {
        blob,
        offset: offset + len,
    })
}

pub fn plain(status_code: u16, text: &str) -> HttpResponse {
    HttpResponse {
        status_code,
        headers: vec![("content-type".into(), "text/plain".into())],
        body: text.as_bytes().to_vec(),
        streaming_strategy: None,
        upgrade: None,
    }
}

#[cfg(test)]
mod tests;
