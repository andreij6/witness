use crate::proof::{Trust, verify_key_json, verify_record_json, verify_shards_json};
use crate::{bundle, case, check};
use wasm_bindgen::prelude::*;

fn js<T>(r: crate::Result<T>) -> Result<T, JsError> {
    r.map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[wasm_bindgen(js_name = queryPlan)]
pub fn query_plan(bytes: &[u8], options: &str) -> Result<String, JsError> {
    js(check::plan_json(bytes, options))
}

#[wasm_bindgen]
pub fn judge(bytes: &[u8], options: &str, fetched: &str) -> Result<String, JsError> {
    js(check::judge_json(bytes, options, fetched))
}

fn trust<'a>(root_key: &'a [u8], canister: &'a [u8], now_ns: u64, max_age_ns: u64) -> Trust<'a> {
    Trust {
        root_key,
        canister,
        now_ns,
        max_age_ns,
    }
}

#[wasm_bindgen(js_name = verifyCertifiedRecord)]
pub fn verify_certified_record(
    record: &str,
    root_key: &[u8],
    canister_id: &[u8],
    now_ns: u64,
    max_age_ns: u64,
) -> Result<String, JsError> {
    js(verify_record_json(
        record,
        &trust(root_key, canister_id, now_ns, max_age_ns),
    ))
}

#[wasm_bindgen(js_name = verifyCertifiedKey)]
pub fn verify_certified_key(
    key: &str,
    root_key: &[u8],
    canister_id: &[u8],
    now_ns: u64,
    max_age_ns: u64,
) -> Result<String, JsError> {
    js(verify_key_json(
        key,
        &trust(root_key, canister_id, now_ns, max_age_ns),
    ))
}

#[wasm_bindgen(js_name = verifyShards)]
pub fn verify_shards(
    shards: &str,
    root_key: &[u8],
    canister_id: &[u8],
    now_ns: u64,
    max_age_ns: u64,
) -> Result<String, JsError> {
    js(verify_shards_json(
        shards,
        &trust(root_key, canister_id, now_ns, max_age_ns),
    ))
}

#[wasm_bindgen(js_name = decodeClaim)]
pub fn decode_claim(claim_cbor: &[u8]) -> Result<String, JsError> {
    js(crate::render::claim_cbor_json(claim_cbor))
}

#[wasm_bindgen(js_name = parseCaseBundle)]
pub fn parse_case_bundle(bytes: &[u8]) -> Result<String, JsError> {
    js(bundle::parse_json(bytes))
}

#[wasm_bindgen(js_name = CaseOpener)]
pub struct JsCaseOpener(Option<case::CaseOpener>);

#[wasm_bindgen(js_class = CaseOpener)]
impl JsCaseOpener {
    #[wasm_bindgen(constructor)]
    pub fn new(
        fragment: &str,
        case_id: &str,
        count: u32,
        bundle_hash: &[u8],
    ) -> Result<JsCaseOpener, JsError> {
        js(case::CaseOpener::new(fragment, case_id, count, bundle_hash).map(|o| Self(Some(o))))
    }

    fn opener(&mut self) -> Result<&mut case::CaseOpener, JsError> {
        self.0
            .as_mut()
            .ok_or_else(|| JsError::new("case opener already finished"))
    }

    pub fn add(&mut self, ix: u32, chunk: &[u8]) -> Result<(), JsError> {
        js(self.opener()?.add(ix, chunk))
    }

    pub fn missing(&mut self) -> Result<Vec<u32>, JsError> {
        Ok(self.opener()?.missing())
    }

    pub fn finish(&mut self) -> Result<Vec<u8>, JsError> {
        self.opener()?;
        js(self.0.take().map_or(
            Err(crate::Error::Case("finished")),
            case::CaseOpener::finish,
        ))
    }
}
