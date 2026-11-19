#[path = "../../../../canisters/registry/tests/support/attest.rs"]
mod attest;
#[path = "../../../../tools/witness-smoke/src/capture.rs"]
#[allow(dead_code)]
mod capture;
#[path = "../../../../tools/witness-smoke/src/device.rs"]
#[allow(dead_code)]
mod device;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use candid::Principal;
use capture::{recompress, seal, submission, template};
use device::{Saved, SmokeSigner};
use futures::executor::block_on;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use witness_android::{KeystoreSigner, Witness, WitnessConfig, WitnessError};
use witness_canister_common::{MediaErr, SubmitResult, UploadTarget};
use witness_core::SealedCapture;
use witness_core::format::{BlurClass, CaseId, Edit, Kind, RecordId, Rect};
use witness_core::image::{Rgb8, encode_jpeg};
use witness_verdict::testkit::photo;
use witness_web::bundle::{CaseBundle, CaseItem};
use witness_web::testkit::seal_case;

type R<T> = Result<T, String>;

const WAIT: Duration = Duration::from_secs(90);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn sha256(b: &[u8]) -> Vec<u8> {
    Sha256::digest(b).to_vec()
}

fn root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    p.canonicalize().unwrap_or(p)
}

fn read_json(path: &Path) -> R<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(err)
}

fn canister(ids: &Value, name: &str) -> R<Principal> {
    let text = ids["canisters"][name]["id"]
        .as_str()
        .ok_or(format!("no {name} id"))?;
    Principal::from_text(text).map_err(err)
}

fn url(ids: &Value, id: &Principal, path: &str) -> R<String> {
    let gateway = ids["gateway"].as_str().ok_or("no gateway")?;
    let (scheme, host) = gateway.split_once("://").ok_or("bad gateway")?;
    let port = host.rsplit_once(':').map_or("", |(_, p)| p);
    Ok(format!("{scheme}://{id}.localhost:{port}{path}"))
}

fn poll<T>(mut f: impl FnMut() -> R<Option<T>>) -> R<T> {
    let start = Instant::now();
    loop {
        if let Some(v) = f()? {
            return Ok(v);
        }
        if start.elapsed() > WAIT {
            return Err("timed out".into());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn pixelate(image: &mut Rgb8, r: Rect) {
    let block = 12u32;
    let (w, h) = image.dimensions();
    for by in (r.y..(r.y + r.h).min(h)).step_by(block as usize) {
        for bx in (r.x..(r.x + r.w).min(w)).step_by(block as usize) {
            let p = image.pixel(bx, by).unwrap_or([128; 3]);
            for y in by..(by + block).min(r.y + r.h).min(h) {
                let row = image.row_mut(y);
                for x in bx..(bx + block).min(r.x + r.w).min(w) {
                    row[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&p);
                }
            }
        }
    }
}

struct Seeder {
    w: Arc<Witness>,
    signer: Arc<dyn KeystoreSigner>,
    key_id: Vec<u8>,
}

impl Seeder {
    fn submit(&self, s: &SealedCapture) -> R<(String, u64)> {
        match block_on(self.w.submit(vec![submission(s)]))
            .map_err(err)?
            .as_slice()
        {
            [SubmitResult::Ok(a) | SubmitResult::Dup(a)] => Ok((a.record_id.clone(), a.record_ix)),
            other => Err(format!("submit: {other:?}")),
        }
    }

    fn upload(&self, target: UploadTarget, bytes: &[u8]) -> R<String> {
        let size = u64::try_from(bytes.len()).map_err(err)?;
        let begun = poll(|| {
            match block_on(self.w.begin_upload(
                0,
                target.clone(),
                size,
                sha256(bytes),
                self.key_id.clone(),
                self.signer.clone(),
                None,
            )) {
                Ok(b) => Ok(Some(b)),
                Err(WitnessError::Media {
                    err: MediaErr::RetryAfterNs(_) | MediaErr::Unauthorized,
                }) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })?;
        for ix in begun.missing {
            block_on(self.w.put_chunk(0, begun.upload, ix, bytes.to_vec())).map_err(err)?;
        }
        block_on(self.w.commit(0, begun.upload)).map_err(err)
    }
}

fn run() -> R<Value> {
    let root = root();
    let out = root.join(".icp-local");
    let ids = read_json(&out.join("ids.json"))?;
    let saved: Saved =
        serde_json::from_value(read_json(&out.join("smoke-device.json")).map_err(|e| {
            format!("{e} (run scripts/smoke-local.sh once to enroll the dev device)")
        })?)
        .map_err(err)?;
    let enrolled = saved.enrolled().map_err(err)?;
    let (dev, anchor) = device::device(attest::device_key(saved.seed), &enrolled)?;
    let now_ns = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos(),
    )
    .map_err(err)?;
    let now_ms = now_ns / 1_000_000;
    let (registry, pages, media) = (
        canister(&ids, "registry")?,
        canister(&ids, "pages")?,
        canister(&ids, "media_0")?,
    );
    let web = canister(&ids, "web")?;
    let w = Witness::new(WitnessConfig {
        replica_url: ids["gateway"].as_str().ok_or("no gateway")?.to_owned(),
        registry,
        local: true,
        ca_root: None,
        identity_seed: Some(sha256(&now_ns.to_be_bytes())),
    })
    .map_err(err)?;
    let s = Seeder {
        w,
        signer: Arc::new(SmokeSigner(attest::device_key(saved.seed))),
        key_id: enrolled.key_id.clone(),
    };

    let capture_id = now_ns >> 8;
    let scene = photo(480, 360, now_ns);
    let original = seal(
        &dev,
        &anchor,
        &scene,
        template(&dev, capture_id, Kind::Original, now_ms, now_ms),
    )
    .map_err(err)?;
    let (orig_id, _) = s.submit(&original)?;
    let parent = RecordId::parse(&orig_id).map_err(err)?;
    let root_capture = original.claim.capture_id;

    let resized = capture::downscale(&scene, 320, 240).map_err(err)?;
    let resize = Kind::Derivative {
        parent,
        root: root_capture,
        edits: vec![Edit::Resize {
            w: 320,
            h: 240,
            filter: 0,
        }],
    };
    let post = seal(
        &dev,
        &anchor,
        &resized,
        template(&dev, capture_id ^ 1, resize, now_ms + 1, now_ms),
    )
    .map_err(err)?;
    let (post_id, _) = s.submit(&post)?;

    let face = Rect {
        x: 300,
        y: 60,
        w: 96,
        h: 120,
    };
    let mut blurred = scene.clone();
    pixelate(&mut blurred, face);
    let blur = Kind::Derivative {
        parent,
        root: root_capture,
        edits: vec![Edit::Blur {
            rect: face,
            class: BlurClass::Face,
            sigma_e2: 800,
        }],
    };
    let edited = seal(
        &dev,
        &anchor,
        &blurred,
        template(&dev, capture_id ^ 2, blur, now_ms + 2, now_ms),
    )
    .map_err(err)?;
    let (edited_id, _) = s.submit(&edited)?;

    let post_preview = s.upload(UploadTarget::Preview(post_id.clone()), &post.jpeg)?;
    let edited_preview = s.upload(UploadTarget::Preview(edited_id.clone()), &edited.jpeg)?;

    let items: Vec<(&SealedCapture, String)> = vec![
        (&original, orig_id.clone()),
        (&post, post_id.clone()),
        (&edited, edited_id.clone()),
    ];
    let bundle = CaseBundle {
        title: "Fender bender",
        summary: Some("Seeded on the local replica: an original, a resized post and a copy with one face blurred."),
        items: items
            .iter()
            .map(|(c, id)| {
                Ok(CaseItem {
                    record_id: RecordId::parse(id).map_err(err)?,
                    jpeg: &c.jpeg,
                    submission: Some(c.submission.clone()),
                })
            })
            .collect::<R<Vec<_>>>()?,
    }
    .to_cbor();
    let mut random = [0u8; CaseId::LEN];
    random[1..].copy_from_slice(&sha256(&(now_ns ^ 0x5eed).to_be_bytes())[..CaseId::LEN - 1]);
    let case_id = CaseId::from_bytes(random);
    let key: [u8; 32] = sha256(&(now_ns ^ 0xca5e).to_be_bytes())
        .try_into()
        .map_err(|_| "key")?;
    let chunks = seal_case(&key, &case_id, &bundle, (now_ns & 0xff) as u8);
    let count = u32::try_from(chunks.len()).map_err(err)?;
    for (ix, chunk) in chunks.iter().enumerate() {
        let ix = u32::try_from(ix).map_err(err)?;
        s.upload(
            UploadTarget::CaseChunk {
                case_id: case_id.to_string(),
                ix,
                count,
            },
            chunk,
        )?;
    }
    let fragment = format!(
        "k={}&h={}",
        URL_SAFE_NO_PAD.encode(key),
        URL_SAFE_NO_PAD.encode(sha256(&bundle))
    );

    let files = out.join("web-seed");
    std::fs::create_dir_all(&files).map_err(err)?;
    std::fs::write(files.join("original.jpg"), &original.jpeg).map_err(err)?;
    std::fs::write(
        files.join("recompressed.jpg"),
        recompress(&original.jpeg, 80).map_err(err)?,
    )
    .map_err(err)?;
    std::fs::write(
        files.join("unrelated.jpg"),
        encode_jpeg(&photo(480, 360, now_ns ^ 0xdead), 90).map_err(err)?,
    )
    .map_err(err)?;

    Ok(json!({
        "original": orig_id,
        "post": post_id,
        "edited": edited_id,
        "case_id": case_id.to_string(),
        "case_chunks": count,
        "bundle_bytes": bundle.len(),
        "previews": [post_preview, edited_preview],
        "urls": {
            "check": url(&ids, &web, "/check")?,
            "post_web": url(&ids, &web, &format!("/p/{post_id}"))?,
            "edited_web": url(&ids, &web, &format!("/p/{edited_id}"))?,
            "post_pages": url(&ids, &pages, &format!("/p/{post_id}"))?,
            "edited_pages": url(&ids, &pages, &format!("/p/{edited_id}"))?,
            "case_web": url(&ids, &web, &format!("/v/{case_id}#{fragment}"))?,
            "case_pages": url(&ids, &pages, &format!("/v/{case_id}#{fragment}"))?,
            "media": url(&ids, &media, "/")?,
        },
        "files": files.display().to_string(),
    }))
}

fn main() {
    match run() {
        Ok(v) => {
            let path = root().join(".icp-local/web-seed.json");
            let text = serde_json::to_string_pretty(&v).unwrap_or_default();
            let _ = std::fs::write(&path, &text);
            println!("{text}");
        }
        Err(e) => {
            eprintln!("web seed failed: {e}");
            std::process::exit(1);
        }
    }
}
