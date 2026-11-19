use candid::{Decode, Encode, Principal};
use ic_agent::Agent;
use ic_http_certification::{HttpRequest, HttpResponse};
use ic_response_verification::verify_request_response_pair;
use serde_bytes::ByteBuf;
use serde_json::Value;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use tokio::runtime::{Builder, Runtime};
use witness_canister_common::{CertifiedKey, CertifiedRecord, ShardInfo, Shards};
use witness_core::format::cbor;
use witness_proof::proof::{ShardEntry, shard_map_hash};
use witness_proof::{Certified, DEFAULT_MAX_AGE_NS, Trust};

pub type R<T> = Result<T, String>;

pub const MAX_CERT_AGE_NS: u64 = DEFAULT_MAX_AGE_NS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ids {
    pub gateway: String,
    pub registry: Principal,
    pub index: Principal,
    pub pages: Principal,
    pub media: Principal,
    pub web: Principal,
    pub ca_root: Option<Vec<u8>>,
}

fn canister(v: &Value, name: &str) -> R<Principal> {
    let text = v["canisters"][name]["id"]
        .as_str()
        .ok_or_else(|| format!("ids: no canister {name}"))?;
    Principal::from_text(text).map_err(|e| format!("ids: {name}: {e}"))
}

pub fn parse_ids(json: &str) -> R<Ids> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("ids: {e}"))?;
    let gateway = v["gateway"]
        .as_str()
        .ok_or("ids: no gateway")?
        .trim_end_matches('/')
        .to_owned();
    Ok(Ids {
        gateway,
        registry: canister(&v, "registry")?,
        index: canister(&v, "index_0")?,
        pages: canister(&v, "pages")?,
        media: canister(&v, "media_0")?,
        web: canister(&v, "web")?,
        ca_root: match v["ca_root"].as_str() {
            None | Some("") => None,
            Some(h) => Some(hex::decode(h).map_err(|e| format!("ids: ca_root: {e}"))?),
        },
    })
}

pub fn module_script_src(html: &str) -> Option<&str> {
    let tag = "<script type=\"module\" src=\"";
    let start = html.find(tag)? + tag.len();
    let len = html[start..].find('"')?;
    Some(&html[start..start + len])
}

pub fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

pub fn strict_page_csp(csp: &str) -> bool {
    let directives: Vec<&str> = csp.split(';').map(str::trim).collect();
    let has = |d: &str| directives.contains(&d);
    let script = directives.iter().find(|d| d.starts_with("script-src "));
    has("default-src 'none'")
        && has("frame-ancestors 'none'")
        && has("base-uri 'none'")
        && script.is_some_and(|s| {
            !s.contains("'unsafe-eval'") && !s.contains("'unsafe-inline'") && !s.contains('*')
        })
}

pub fn curl_headers(url: &str) -> R<String> {
    let o = Command::new("curl")
        .args(["-sS", "--max-time", "20", "-o", "/dev/null", "-D", "-", url])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "curl {url}: {}",
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

pub fn gateway_url(gateway: &str, canister: Principal, path: &str) -> R<String> {
    let (scheme, host) = gateway
        .split_once("://")
        .ok_or("gateway url has no scheme")?;
    let port = host
        .rsplit_once(':')
        .map(|(_, p)| p)
        .ok_or("gateway url has no port")?;
    Ok(format!("{scheme}://{canister}.localhost:{port}{path}"))
}

fn entries(list: &[ShardInfo]) -> impl ExactSizeIterator<Item = ShardEntry<'_>> {
    list.iter().map(|i| ShardEntry {
        shard: i.shard,
        canister: i.canister.as_slice(),
        open: i.open,
    })
}

pub fn shards_hash(s: &Shards) -> R<[u8; 32]> {
    shard_map_hash(entries(&s.index), entries(&s.media)).map_err(|e| format!("shard map: {e}"))
}

fn certified_json(c: &Certified, mut extra: Value) -> Value {
    extra["certifiedData"] = json!(hex::encode(&c.data));
    extra["certTimeNs"] = json!(c.time_ns.to_string());
    extra
}

pub fn verify_record_at(trust: &Trust<'_>, r: &CertifiedRecord) -> R<Value> {
    let c = trust
        .certified_record(r)
        .map_err(|e| format!("record proof: {e}"))?;
    Ok(certified_json(
        &c,
        json!({"recordId": r.record_id, "recordIx": r.record_ix, "leafCount": r.leaf_count, "mediaShard": r.media_shard}),
    ))
}

fn hexes(list: &[ByteBuf]) -> Vec<String> {
    list.iter().map(hex::encode).collect()
}

fn shard_list(list: &[ShardInfo]) -> Vec<Value> {
    list.iter()
        .map(|i| json!({"shard": i.shard, "canister": hex::encode(i.canister.as_slice()), "open": i.open}))
        .collect()
}

pub fn proof_fixture(
    root_key: &[u8],
    registry: &Principal,
    r: &CertifiedRecord,
    k: &CertifiedKey,
    shards: &Shards,
    bundle: &[u8],
) -> R<Value> {
    let trust = Trust {
        root_key,
        canister: registry.as_slice(),
        now_ns: 0,
        max_age_ns: 0,
    };
    let rc = trust
        .certified_record(r)
        .map_err(|e| format!("record proof: {e}"))?;
    let (info, kc) = trust
        .certified_key(k)
        .map_err(|e| format!("key proof: {e}"))?;
    let sc = trust
        .shards(bundle, &shards_hash(shards)?)
        .map_err(|e| format!("shards proof: {e}"))?;
    Ok(json!({
        "rootKey": hex::encode(root_key),
        "registry": hex::encode(registry.as_slice()),
        "record": {
            "timeNs": rc.time_ns.to_string(),
            "certifiedData": hex::encode(&rc.data),
            "recordIx": r.record_ix.to_string(),
            "recordId": r.record_id,
            "claim": hex::encode(&r.claim),
            "c2paClaim": hex::encode(&r.c2pa_claim),
            "cose": hex::encode(&r.cose),
            "anchoredNs": r.anchored_ns.to_string(),
            "leafCount": r.leaf_count.to_string(),
            "mmrProof": hexes(&r.mmr_proof),
            "certificate": hex::encode(&r.certificate),
            "mediaShard": r.media_shard,
        },
        "key": {
            "timeNs": kc.time_ns.to_string(),
            "certifiedData": hex::encode(&kc.data),
            "info": hex::encode(cbor::to_vec(&info)),
            "smtProof": hex::encode(&k.smt_proof),
            "certificate": hex::encode(&k.certificate),
        },
        "shards": {
            "timeNs": sc.time_ns.to_string(),
            "certifiedData": hex::encode(&sc.data),
            "index": shard_list(&shards.index),
            "media": shard_list(&shards.media),
            "bundle": hex::encode(bundle),
        },
    }))
}

pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub status: u16,
    pub body: Vec<u8>,
}

pub struct Net {
    rt: Runtime,
    agent: Agent,
    pub root_key: Vec<u8>,
}

impl Net {
    pub fn connect(url: &str) -> R<Self> {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let agent = Agent::builder()
            .with_url(url)
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(agent.fetch_root_key())
            .map_err(|e| format!("fetch_root_key: {e}"))?;
        let root_key = agent.read_root_key();
        Ok(Self {
            rt,
            agent,
            root_key,
        })
    }

    fn trust<'a>(&'a self, canister: &'a Principal) -> Trust<'a> {
        Trust {
            root_key: &self.root_key,
            canister: canister.as_slice(),
            now_ns: now_ns(),
            max_age_ns: MAX_CERT_AGE_NS,
        }
    }

    pub fn verify_record(&self, registry: &Principal, r: &CertifiedRecord) -> R<Value> {
        verify_record_at(&self.trust(registry), r)
    }

    pub fn verify_key(&self, registry: &Principal, k: &CertifiedKey) -> R<Value> {
        let (info, c) = self
            .trust(registry)
            .certified_key(k)
            .map_err(|e| format!("key proof: {e}"))?;
        Ok(certified_json(
            &c,
            json!({"keyId": hex::encode(info.key_id.as_bytes())}),
        ))
    }

    pub fn shards(&self, registry: &Principal) -> R<(Shards, Vec<u8>)> {
        let arg = Encode!().map_err(|e| e.to_string())?;
        let bytes = self
            .rt
            .block_on(self.agent.query(registry, "shards").with_arg(arg).call())
            .map_err(|e| format!("shards: {e}"))?;
        let (s, b) = Decode!(&bytes, Shards, ByteBuf).map_err(|e| e.to_string())?;
        Ok((s, b.into_vec()))
    }

    pub fn verify_shards(&self, registry: &Principal, s: &Shards, bundle: &[u8]) -> R<Value> {
        let c = self
            .trust(registry)
            .shards(bundle, &shards_hash(s)?)
            .map_err(|e| format!("shards proof: {e}"))?;
        Ok(certified_json(
            &c,
            json!({"index": s.index.len(), "media": s.media.len()}),
        ))
    }

    pub fn certified_get(&self, canister: Principal, path: &str) -> R<Fetched> {
        let req = HttpRequest::get(path.to_owned())
            .with_certificate_version(2)
            .build();
        let arg = Encode!(&req).map_err(|e| e.to_string())?;
        let bytes = self
            .rt
            .block_on(
                self.agent
                    .query(&canister, "http_request")
                    .with_arg(arg)
                    .call(),
            )
            .map_err(|e| format!("http_request {path}: {e}"))?;
        let resp = Decode!(&bytes, HttpResponse<'static>).map_err(|e| e.to_string())?;
        let info = verify_request_response_pair(
            req,
            resp.clone(),
            canister.as_slice(),
            u128::from(now_ns()),
            u128::from(MAX_CERT_AGE_NS),
            &self.root_key,
            2,
        )
        .map_err(|e| format!("certification of {path}: {e:?}"))?;
        let verified = info.response.ok_or("certificate covers no response")?;
        if verified.body != resp.body() {
            return Err(format!("certified body differs for {path}"));
        }
        Ok(Fetched {
            status: verified.status_code.unwrap_or(resp.status_code().as_u16()),
            body: verified.body,
        })
    }
}

pub fn curl(url: &str, out: &Path) -> R<Fetched> {
    let o = Command::new("curl")
        .args(["-sS", "--max-time", "20", "-o"])
        .arg(out)
        .args(["-w", "%{http_code}", url])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "curl {url}: {}",
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    let status = String::from_utf8_lossy(&o.stdout)
        .trim()
        .parse()
        .map_err(|_| "curl: bad status".to_owned())?;
    let body = std::fs::read(out).unwrap_or_default();
    Ok(Fetched { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: &str = r#"{"network":"local","gateway":"http://127.0.0.1:8090/","canisters":{
        "registry":{"id":"4caro-hl777-77775-aaaba-cai","url":"x"},
        "index_0":{"id":"4fbx2-kt777-77775-aaabq-cai","url":"x"},
        "pages":{"id":"4qggx-l3777-77775-aaaca-cai","url":"x"},
        "media_0":{"id":"4xhad-gd777-77775-aaacq-cai","url":"x"},
        "web":{"id":"46el7-ql777-77775-aaada-cai","url":"x"}}}"#;

    #[test]
    fn parses_ids_file() {
        let ids = parse_ids(IDS).unwrap();
        assert_eq!(ids.gateway, "http://127.0.0.1:8090");
        assert_eq!(ids.registry.to_text(), "4caro-hl777-77775-aaaba-cai");
        assert_eq!(ids.media.to_text(), "4xhad-gd777-77775-aaacq-cai");
        assert_eq!(ids.web.to_text(), "46el7-ql777-77775-aaada-cai");
        assert_eq!(ids.ca_root, None);
        assert_eq!(parse_ids(IDS).unwrap(), ids);
    }

    #[test]
    fn parses_the_pinned_ca_root() {
        let with = |v: &str| IDS.replacen('{', &format!("{{\"ca_root\":{v},"), 1);
        assert_eq!(
            parse_ids(&with("\"30ff\"")).unwrap().ca_root,
            Some(vec![0x30, 0xff])
        );
        assert_eq!(parse_ids(&with("\"\"")).unwrap().ca_root, None);
        assert_eq!(parse_ids(&with("null")).unwrap().ca_root, None);
        assert!(parse_ids(&with("\"zz\"")).is_err());
    }

    #[test]
    fn header_value_is_case_insensitive_and_trimmed() {
        let h = "HTTP/1.1 200 OK\r\nContent-Security-Policy:  default-src 'none'\r\nX-A: b:c\r\n";
        assert_eq!(
            header_value(h, "content-security-policy"),
            Some("default-src 'none'")
        );
        assert_eq!(header_value(h, "x-a"), Some("b:c"));
        assert_eq!(header_value(h, "x-missing"), None);
        assert_eq!(header_value("", "x-a"), None);
    }

    #[test]
    fn strict_page_csp_requires_lockdown_directives() {
        let good = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; base-uri 'none'; frame-ancestors 'none'";
        assert!(strict_page_csp(good));
        for bad in [
            "",
            "script-src 'self'; base-uri 'none'; frame-ancestors 'none'",
            "default-src 'none'; script-src 'self'; base-uri 'none'",
            "default-src 'none'; script-src 'self'; frame-ancestors 'none'",
            "default-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            "default-src 'none'; script-src 'self' 'unsafe-eval'; base-uri 'none'; frame-ancestors 'none'",
            "default-src 'none'; script-src 'self' 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
            "default-src 'none'; script-src *; base-uri 'none'; frame-ancestors 'none'",
            "default-src 'none'; script-src https://*.icp0.io; base-uri 'none'; frame-ancestors 'none'",
        ] {
            assert!(!strict_page_csp(bad), "{bad}");
        }
    }

    #[test]
    fn rejects_bad_ids() {
        assert!(parse_ids("").is_err());
        assert!(parse_ids("{}").is_err());
        assert!(parse_ids(&IDS.replace("4caro-hl777-77775-aaaba-cai", "nope")).is_err());
        assert!(parse_ids(&IDS.replace("\"pages\"", "\"pagez\"")).is_err());
        assert!(parse_ids(&IDS.replace("\"web\"", "\"webb\"")).is_err());
    }

    #[test]
    fn finds_the_module_script_bundle() {
        let html = r#"<head></head><body><script type="application/json" id="x">{}</script><div id="app"></div><script type="module" src="http://web.localhost:8090/boot.mjs"></script></body>"#;
        assert_eq!(
            module_script_src(html),
            Some("http://web.localhost:8090/boot.mjs")
        );
        assert_eq!(module_script_src("<script src=\"x.js\"></script>"), None);
        assert_eq!(
            module_script_src("<script type=\"module\" src=\"unterminated"),
            None
        );
        assert_eq!(module_script_src(""), None);
    }

    #[test]
    fn builds_gateway_urls() {
        let p = Principal::from_text("4qggx-l3777-77775-aaaca-cai").unwrap();
        assert_eq!(
            gateway_url("http://127.0.0.1:8090", p, "/p/ABC").unwrap(),
            "http://4qggx-l3777-77775-aaaca-cai.localhost:8090/p/ABC"
        );
        assert!(gateway_url("127.0.0.1:8090", p, "/").is_err());
        assert!(gateway_url("http://localhost", p, "/").is_err());
    }

    fn forged() -> CertifiedRecord {
        CertifiedRecord {
            record_ix: 0,
            record_id: "0000000000000000".into(),
            claim: vec![],
            c2pa_claim: vec![],
            cose: vec![],
            anchored_ns: 0,
            mmr_proof: vec![],
            leaf_count: 1,
            certificate: vec![],
            media_shard: 0,
        }
    }

    #[test]
    fn forged_record_fails_proof_without_network() {
        let p = Principal::anonymous();
        let trust = Trust {
            root_key: &[],
            canister: p.as_slice(),
            now_ns: 0,
            max_age_ns: 0,
        };
        let e = verify_record_at(&trust, &forged()).unwrap_err();
        assert!(e.starts_with("record proof: "), "{e}");
        assert!(proof_fixture(&[], &p, &forged(), &forged_key(), &Shards::default(), &[]).is_err());
    }

    fn forged_key() -> CertifiedKey {
        CertifiedKey {
            key: witness_canister_common::KeyInfo {
                key_id: vec![1; 16],
                pubkey: vec![2],
                security: witness_canister_common::Security::Tee,
                provisioning: witness_canister_common::Prov::Rkp,
                freshness: witness_canister_common::Fresh::Fresh,
                patch: 0,
                enrolled_ns: 0,
                active_until_ns: 0,
                history: vec![],
                chain_serials: vec![],
            },
            smt_proof: vec![],
            certificate: vec![],
        }
    }

    #[test]
    fn shard_hash_matches_the_shared_verifier_and_bounds_ids() {
        let info = |shard, b| ShardInfo {
            shard,
            canister: Principal::from_slice(&[b; 10]),
            open: shard == 0,
        };
        let s = Shards {
            index: vec![info(0, 1), info(1, 2)],
            media: vec![info(0, 3)],
        };
        let e = |shard, b: &'static [u8], open| ShardEntry {
            shard,
            canister: b,
            open,
        };
        assert_eq!(
            shards_hash(&s).unwrap(),
            shard_map_hash(
                [e(0, &[1; 10], true), e(1, &[2; 10], false)],
                [e(0, &[3; 10], true)]
            )
            .unwrap()
        );
        assert_ne!(
            shards_hash(&s).unwrap(),
            shards_hash(&Shards::default()).unwrap()
        );
    }

    #[test]
    fn now_is_after_2025() {
        assert!(now_ns() > 1_735_689_600_000_000_000);
    }

    #[test]
    fn curl_reports_unreachable_hosts() {
        let out = std::env::temp_dir().join("witness-smoke-curl-test");
        assert!(curl("http://127.0.0.1:9/", &out).is_err());
    }
}
