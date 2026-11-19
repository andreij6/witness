#[path = "../../../canisters/registry/tests/support/attest.rs"]
mod attest;
mod capture;
mod device;
mod net;
mod steps;

use crate::capture::{
    derived_from, exact_hashes, preview, probe_hashes, recompress, seal, submission, template,
};
use crate::device::{Saved, SmokeSigner, enrollment};
use crate::net::{
    Fetched, Ids, Net, R, curl, curl_headers, gateway_url, header_value, module_script_src, now_ns,
    parse_ids, proof_fixture, strict_page_csp,
};
use crate::steps::{Steps, poll};
use futures::executor::block_on;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use witness_android::{KeystoreSigner, Witness, WitnessConfig, WitnessError, case_info};
use witness_canister_common::{MediaErr, ProbeResult, SubmitResult, UploadTarget};
use witness_core::SealedCapture;
use witness_core::format::{CaseId, Kind, Outcome};
use witness_verdict::testkit::photo;

const WAIT: Duration = Duration::from_secs(60);

fn root() -> PathBuf {
    let p = std::env::var_os("WITNESS_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    p.canonicalize().unwrap_or(p)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn anchored(r: &[SubmitResult]) -> R<witness_canister_common::Anchored> {
    match r {
        [SubmitResult::Ok(a)] => Ok(a.clone()),
        other => Err(format!("submit: {other:?}")),
    }
}

fn probe(w: &Witness, s: &SealedCapture) -> R<ProbeResult> {
    block_on(w.probe(
        probe_hashes(&s.fingerprints),
        vec![s.claim.capture_id.to_u64()],
        exact_hashes(s),
    ))
    .map_err(err)
}

fn found_by(p: &ProbeResult, ix: u64) -> (bool, bool, bool) {
    (
        p.pdq.iter().any(|h| h.record_ix == ix && h.distance == 0),
        p.watermark.contains(&ix),
        p.exact.contains(&ix),
    )
}

fn sha256(b: &[u8]) -> Vec<u8> {
    Sha256::digest(b).to_vec()
}

fn load_saved(path: &Path) -> Option<Saved> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn expect_status(f: &Fetched, status: u16, what: &str) -> R<()> {
    if f.status == status {
        Ok(())
    } else {
        Err(format!("{what}: status {} (want {status})", f.status))
    }
}

fn run(steps: &mut Steps, ids: &Ids, out: &Path) -> R<()> {
    let run_ns = now_ns();
    let now_ms = run_ns / 1_000_000;
    let net = steps.step("connect + fetch_root_key", || Net::connect(&ids.gateway))?;
    let client = |seed: u64| {
        Witness::new(WitnessConfig {
            replica_url: ids.gateway.clone(),
            registry: ids.registry,
            local: true,
            ca_root: ids.ca_root.clone(),
            identity_seed: Some(sha256(&seed.to_be_bytes())),
            max_cert_age_s: None,
        })
        .map_err(err)
    };
    let w = steps.step("witness client", || client(run_ns))?;
    steps.check("registry ca_root matches the pinned anchor", || {
        let root = block_on(w.ca_root()).map_err(err)?.ok_or("no CA root")?;
        match &ids.ca_root {
            Some(pinned) if *pinned != root => Err("ids.json ca_root differs from registry".into()),
            Some(_) => Ok(json!("pinned")),
            None => Ok(json!("unpinned (dev query)")),
        }
    })?;
    let challenge = steps.step("challenge", || block_on(w.challenge()).map_err(err))?;
    let saved_path = out.join("smoke-device.json");
    let fresh = std::env::var_os("WITNESS_SMOKE_FRESH_DEVICE").is_some();
    let saved = steps.step("enroll dev-attestation device", || {
        let reuse = load_saved(&saved_path)
            .filter(|_| !fresh)
            .and_then(|s| Some((s.enrolled().ok()?, s)))
            .filter(|(e, _)| {
                block_on(w.get_key(e.key_id.clone()))
                    .ok()
                    .flatten()
                    .is_some()
            });
        if let Some((_, s)) = reuse {
            return Ok((s, false));
        }
        let seed = run_ns | 1;
        let now_s = i64::try_from(run_ns / 1_000_000_000).map_err(err)?;
        let e = block_on(w.enroll(enrollment(&challenge, seed, now_s))).map_err(err)?;
        let s = Saved::new(seed, &e);
        std::fs::write(&saved_path, serde_json::to_vec_pretty(&s).map_err(err)?).map_err(err)?;
        Ok((s, true))
    })?;
    let (saved, enrolled_now) = saved;
    steps.note(
        "device",
        json!({"key_id": saved.key_id, "enrolled_this_run": enrolled_now}),
    );
    let enrolled = saved.enrolled().map_err(err)?;
    let (dev, anchor) = device::device(attest::device_key(saved.seed), &enrolled)?;
    let signer: Arc<dyn KeystoreSigner> = Arc::new(SmokeSigner(attest::device_key(saved.seed)));
    steps.check("get_key + verify certificate/SMT proof", || {
        let k = block_on(w.get_key(enrolled.key_id.clone()))
            .map_err(err)?
            .ok_or("key not found")?;
        net.verify_key(&ids.registry, &k)
    })?;

    let image = photo(640, 480, run_ns);
    let capture_id = run_ns >> 8;
    let original = steps.step("seal original", || {
        seal(
            &dev,
            &anchor,
            &image,
            template(&dev, capture_id, Kind::Original, now_ms, now_ms),
        )
        .map_err(err)
    })?;
    let orig = steps.step("submit original", || {
        anchored(&block_on(w.submit(vec![submission(&original)])).map_err(err)?)
    })?;
    steps.note(
        "original",
        json!({"record_id": orig.record_id, "record_ix": orig.record_ix}),
    );
    steps.check("get + verify certificate/MMR proof", || {
        let r = block_on(w.get(orig.record_id.clone()))
            .map_err(err)?
            .ok_or("record not found")?;
        net.verify_record(&ids.registry, &r)
    })?;
    steps.check("shards + verify certificate/SMT proof", || {
        let (s, bundle) = net.shards(&ids.registry)?;
        net.verify_shards(&ids.registry, &s, &bundle)
    })?;
    if let Some(path) = std::env::var_os("WITNESS_SMOKE_PROOF_FIXTURE") {
        steps.check("capture proof fixture", || {
            let r = block_on(w.get(orig.record_id.clone()))
                .map_err(err)?
                .ok_or("record not found")?;
            let k = block_on(w.get_key(enrolled.key_id.clone()))
                .map_err(err)?
                .ok_or("key not found")?;
            let (s, bundle) = net.shards(&ids.registry)?;
            let fixture = proof_fixture(&net.root_key, &ids.registry, &r, &k, &s, &bundle)?;
            std::fs::write(&path, serde_json::to_vec_pretty(&fixture).map_err(err)?)
                .map_err(err)?;
            Ok(json!({"path": PathBuf::from(&path).display().to_string()}))
        })?;
    }
    steps.check("media_shard routes the record to media_0", || {
        let r = block_on(w.get(orig.record_id.clone()))
            .map_err(err)?
            .ok_or("record not found")?;
        if r.media_shard != orig.media_shard {
            return Err(format!(
                "get {} != submit {}",
                r.media_shard, orig.media_shard
            ));
        }
        let shards = block_on(w.shards()).map_err(err)?;
        let media = shards
            .media_shard(r.media_shard)
            .ok_or("media_shard not in shard map")?;
        (media.canister == ids.media)
            .then(|| json!({"media_shard": r.media_shard, "canister": media.canister.to_text()}))
            .ok_or_else(|| format!("routes to {}", media.canister))
    })?;
    steps.check("index probe finds original (pdq + watermark + exact)", || {
        poll(WAIT, || {
            let p = probe(&w, &original)?;
            let f = found_by(&p, orig.record_ix);
            Ok((f == (true, true, true)).then(|| json!({"pdq": f.0, "watermark": f.1, "exact": f.2, "indexed_upto": p.indexed_upto})))
        })
    })?;
    steps.check("judge sealed file = ORIGINAL", || {
        let v = block_on(w.check(original.jpeg.clone(), None)).map_err(err)?;
        let anchor_ok = v.credentials_judged && v.dev_anchor == ids.ca_root.is_none();
        (v.outcome == Outcome::Original && anchor_ok && !v.proofs_rejected)
            .then(|| json!({"outcome": format!("{:?}", v.outcome), "flags": format!("{:?}", v.flags), "dev_anchor": v.dev_anchor}))
            .ok_or_else(|| format!("verdict {v:?}"))
    })?;
    steps.check("judge recompressed copy = MATCHES_WITH_CHANGES", || {
        let copy = recompress(&original.jpeg, 80).map_err(err)?;
        let v = block_on(w.check(copy, None)).map_err(err)?;
        (v.outcome == Outcome::MatchesWithChanges && !v.proofs_rejected)
            .then(|| json!({"outcome": format!("{:?}", v.outcome), "flags": format!("{:?}", v.flags), "regions": v.regions.len()}))
            .ok_or_else(|| format!("verdict {v:?}"))
    })?;

    let share = capture::downscale(&image, 480, 360).map_err(err)?;
    let derivative = steps.step("seal derivative", || {
        let kind = derived_from(&original.record_id, original.claim.capture_id);
        seal(
            &dev,
            &anchor,
            &share,
            template(&dev, capture_id ^ 1, kind, now_ms + 1, now_ms),
        )
        .map_err(err)
    })?;
    let der = steps.step("submit derivative", || {
        anchored(&block_on(w.submit(vec![submission(&derivative)])).map_err(err)?)
    })?;
    steps.note(
        "derivative",
        json!({"record_id": der.record_id, "record_ix": der.record_ix}),
    );
    let page = format!("/p/{}", der.record_id);
    let page_url = gateway_url(&ids.gateway, ids.pages, &page)?;
    steps.check("pages http_request /p/{id} certified 200", || {
        poll(WAIT, || {
            let f = net.certified_get(ids.pages, &page)?;
            Ok((f.status == 200).then(|| json!({"status": f.status, "bytes": f.body.len()})))
        })
    })?;
    steps.check("curl gateway /p/{id} = 200", || {
        let f = curl(&page_url, &out.join("smoke-page.html"))?;
        expect_status(&f, 200, &page_url)?;
        if !String::from_utf8_lossy(&f.body).contains(&der.record_id) {
            return Err("page does not mention the record id".into());
        }
        Ok(json!({"url": page_url, "bytes": f.body.len()}))
    })?;
    steps.check(
        "/p/{id} loads the bundle from the local web canister",
        || {
            let f = net.certified_get(ids.pages, &page)?;
            expect_status(&f, 200, &page)?;
            let html = String::from_utf8_lossy(&f.body);
            let want = gateway_url(&ids.gateway, ids.web, "/boot.mjs")?;
            let src = module_script_src(&html).ok_or("page has no module script")?;
            if src != want {
                return Err(format!("bundle url {src} (want {want})"));
            }
            let boot = curl(src, &out.join("smoke-boot.mjs"))?;
            expect_status(&boot, 200, src)?;
            Ok(json!({"bundle": src, "boot_bytes": boot.body.len()}))
        },
    )?;
    steps.check("web app pages send a strict CSP", || {
        let url = gateway_url(&ids.gateway, ids.web, "/check")?;
        let headers = curl_headers(&url)?;
        let csp = header_value(&headers, "content-security-policy").ok_or("no CSP")?;
        if !strict_page_csp(csp) {
            return Err(format!("weak CSP: {csp}"));
        }
        if header_value(&headers, "x-content-type-options") != Some("nosniff") {
            return Err("missing nosniff".into());
        }
        Ok(json!({"url": url, "csp": csp}))
    })?;

    let preview_jpeg = preview(&share, 80).map_err(err)?;
    let preview_size = u64::try_from(preview_jpeg.len()).map_err(err)?;
    let begin_preview = |who: &Witness| {
        block_on(who.begin_upload(
            der.media_shard,
            UploadTarget::Preview(der.record_id.clone()),
            preview_size,
            sha256(&preview_jpeg),
            enrolled.key_id.clone(),
            signer.clone(),
            None,
        ))
    };
    let begun = steps.step("media begin preview upload", || {
        poll(WAIT, || match begin_preview(&w) {
            Ok(b) => Ok(Some(b)),
            Err(WitnessError::Media {
                err: MediaErr::RetryAfterNs(_) | MediaErr::Unauthorized,
            }) => Ok(None),
            Err(e) => Err(e.to_string()),
        })
    })?;
    steps.check(
        "replayed BeginUpload from another caller cannot take over",
        || {
            let thief = client(run_ns ^ 0x5A5A)?;
            let wait = match begin_preview(&thief) {
                Err(WitnessError::Media {
                    err: MediaErr::RetryAfterNs(wait),
                }) => wait,
                other => return Err(format!("begin by another caller: {other:?}")),
            };
            let bytes = preview_jpeg.clone();
            match block_on(thief.put_chunk(der.media_shard, begun.upload, 0, bytes)) {
                Err(WitnessError::Media {
                    err: MediaErr::Unauthorized,
                }) => Ok(json!({"begin_retry_after_s": wait / 1_000_000_000, "put_chunk": "unauthorized"})),
                other => Err(format!("put_chunk by another caller: {other:?}")),
            }
        },
    )?;
    let preview_path = steps.step("media upload preview", || {
        for ix in &begun.missing {
            block_on(w.put_chunk(der.media_shard, begun.upload, *ix, preview_jpeg.clone()))
                .map_err(err)?;
        }
        block_on(w.commit(der.media_shard, begun.upload)).map_err(err)
    })?;
    steps.check("case chunk upload + case_info expiry", || {
        let case = CaseId::new(0, sha256(&run_ns.to_le_bytes())[..15].try_into().map_err(err)?);
        let chunk = sha256(&run_ns.to_be_bytes()).repeat(8);
        let size = u64::try_from(chunk.len()).map_err(err)?;
        let target = UploadTarget::CaseChunk {
            case_id: case.to_string(),
            ix: 0,
            count: 1,
        };
        let begun = match block_on(w.begin_upload(
            case.shard(),
            target,
            size,
            sha256(&chunk),
            enrolled.key_id.clone(),
            signer.clone(),
            None,
        )) {
            Err(WitnessError::Media {
                err: MediaErr::QuotaExceeded,
            }) => {
                return Ok(json!({"skipped": "daily case quota used; set WITNESS_SMOKE_FRESH_DEVICE=1"}));
            }
            other => other.map_err(err)?,
        };
        block_on(w.put_chunk(case.shard(), begun.upload, 0, chunk)).map_err(err)?;
        block_on(w.commit(case.shard(), begun.upload)).map_err(err)?;
        let info = block_on(case_info(w.clone(), case.shard(), case.to_string()))
            .map_err(err)?
            .ok_or("case_info missing")?;
        let ttl_days = info.expires_ns.saturating_sub(run_ns) / 86_400_000_000_000;
        if (info.bytes, info.chunks) != (size, 1) || !(29..=30).contains(&ttl_days) {
            return Err(format!("case_info {info:?}"));
        }
        let unknown = CaseId::new(0, [0; 15]).to_string();
        if block_on(case_info(w.clone(), 0, unknown)).map_err(err)?.is_some() {
            return Err("unknown case has info".into());
        }
        Ok(json!({"case_id": case.to_string(), "bytes": info.bytes, "chunks": info.chunks, "expires_in_days": ttl_days}))
    })?;
    let media_url = gateway_url(&ids.gateway, ids.media, &preview_path)?;
    steps.check("media certified fetch of preview", || {
        let f = net.certified_get(ids.media, &preview_path)?;
        expect_status(&f, 200, &preview_path)?;
        (f.body == preview_jpeg)
            .then(|| json!({"path": preview_path, "bytes": f.body.len()}))
            .ok_or_else(|| "preview bytes differ".to_owned())
    })?;
    steps.check("curl gateway preview = uploaded bytes", || {
        let f = curl(&media_url, &out.join("smoke-preview.jpg"))?;
        expect_status(&f, 200, &media_url)?;
        (f.body == preview_jpeg)
            .then(|| json!({"url": media_url, "bytes": f.body.len()}))
            .ok_or_else(|| "gateway preview bytes differ".to_owned())
    })?;

    steps.check("erase_mine derivative + original", || {
        let nonce = run_ns.to_be_bytes().to_vec();
        block_on(w.erase_mine(der.record_id.clone(), nonce.clone(), signer.clone()))
            .map_err(err)?;
        block_on(w.erase_mine(orig.record_id.clone(), nonce, signer.clone())).map_err(err)?;
        Ok(json!("ok"))
    })?;
    steps.check("index probe no longer finds them", || {
        poll(WAIT, || {
            let a = found_by(&probe(&w, &original)?, orig.record_ix);
            let b = found_by(&probe(&w, &derivative)?, der.record_ix);
            Ok((a == (false, false, false) && b == (false, false, false)).then(|| json!("gone")))
        })
    })?;
    steps.check("pages /p/{id} certified 410 tombstone", || {
        poll(WAIT, || {
            let f = net.certified_get(ids.pages, &page)?;
            Ok((f.status == 410).then(|| json!({"status": f.status})))
        })
    })?;
    steps.check("curl gateway /p/{id} = 410", || {
        let f = curl(&page_url, &out.join("smoke-page-gone.html"))?;
        expect_status(&f, 410, &page_url)?;
        Ok(json!({"url": page_url}))
    })?;
    steps.check("media preview erased = certified 404", || {
        poll(WAIT, || {
            let f = net.certified_get(ids.media, &preview_path)?;
            Ok((f.status == 404).then(|| json!({"status": f.status})))
        })
    })?;
    steps.check("curl gateway erased preview = 404", || {
        let f = curl(&media_url, &out.join("smoke-preview-gone"))?;
        expect_status(&f, 404, &media_url)?;
        Ok(json!({"url": media_url, "gateway_status": f.status}))
    })?;
    steps.check("permanent record still readable", || {
        let r = block_on(w.get(orig.record_id.clone()))
            .map_err(err)?
            .ok_or("record vanished")?;
        net.verify_record(&ids.registry, &r)
    })?;
    Ok(())
}

fn main() {
    let out = root().join(".icp-local");
    let ids = std::fs::read_to_string(out.join("ids.json"))
        .map_err(|e| format!("read .icp-local/ids.json (run scripts/deploy-local.sh): {e}"))
        .and_then(|s| parse_ids(&s));
    let ids = match ids {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut steps = Steps::default();
    let result = run(&mut steps, &ids, &out);
    let report = steps.report(&ids, result.as_ref().err());
    let _ = std::fs::write(
        out.join("smoke.json"),
        serde_json::to_vec_pretty(&report).unwrap_or_default(),
    );
    println!("{}", steps.table());
    match result {
        Ok(()) => println!(
            "SMOKE OK ({} steps) report: {}",
            steps.len(),
            out.join("smoke.json").display()
        ),
        Err(e) => {
            eprintln!("SMOKE FAILED: {e}");
            std::process::exit(1);
        }
    }
}
