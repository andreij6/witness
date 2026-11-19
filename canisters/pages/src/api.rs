use crate::html::{self, ShellView};
use crate::site::Site;
use crate::store::{Page, PageKind, Store};
use crate::view;
use ic_stable_structures::Memory;
use witness_canister_common::{PageErr, PageResult, Removal, Shell};
use witness_format::{RecordId, Submission, WitnessClaim};

pub const MAX_BATCH: usize = witness_canister_common::limits::MAX_SUBMIT_BATCH;

fn malformed(msg: impl Into<String>) -> PageResult {
    PageResult::Err(PageErr::Malformed(msg.into()))
}

fn key_of(id: &RecordId) -> [u8; 16] {
    id.to_base32()
}

fn text(key: &[u8; 16]) -> String {
    key.iter().map(|b| char::from(*b)).collect()
}

pub fn render(input: &Shell, claim: &WitnessClaim, id: &RecordId, site: &Site) -> String {
    let key = key_of(id);
    let record_id = text(&key);
    let grouped = id.grouped();
    let media = input.media.to_text();
    let headline = view::headline(&claim.kind);
    let changes = view::changes(claim.kind.edits());
    let captured = view::utc(claim.time.wall_ms);
    let anchored = view::utc(input.anchored_ns / 1_000_000);
    let area = claim.coarse.as_ref().map(view::area);
    html::proof_shell(
        &ShellView {
            record_id: &record_id,
            grouped: &grouped,
            media: &media,
            media_shard: input.media_shard,
            headline: &headline,
            changes: changes.as_deref(),
            original: claim.kind.is_original(),
            captured: &captured,
            anchored: &anchored,
            area: area.as_deref(),
            dev: input.dev,
        },
        &site.origins,
    )
}

pub fn put_shell<M: Memory>(store: &mut Store<M>, site: &Site, input: &Shell) -> PageResult {
    if input.claim.len() > Submission::MAX_PART {
        return malformed("claim too large");
    }
    let claim = match WitnessClaim::from_cbor(&input.claim) {
        Ok(c) => c,
        Err(e) => return malformed(format!("claim: {e:?}")),
    };
    let id = RecordId::from_claim_cbor(&input.claim);
    if RecordId::parse(&input.record_id).ok() != Some(id) {
        return malformed("record_id does not match claim");
    }
    let key = key_of(&id);
    if store.get(&key).is_some_and(|p| p.kind.is_gone()) {
        return PageResult::Err(PageErr::Gone);
    }
    let body = render(input, &claim, &id, site).into_bytes();
    let kind = if input.dev {
        PageKind::DevShell
    } else {
        PageKind::Shell
    };
    let resp_hash = site.page_hash(kind, &body);
    store.put(
        key,
        Page {
            kind,
            resp_hash,
            body,
        },
    );
    PageResult::Ok(text(&key))
}

pub fn tombstone<M: Memory>(
    store: &mut Store<M>,
    site: &Site,
    record_id: &str,
    why: Removal,
) -> PageResult {
    let id = match RecordId::parse(record_id) {
        Ok(id) => id,
        Err(e) => return malformed(format!("record_id: {e}")),
    };
    let key = key_of(&id);
    let kind = PageKind::Gone(why);
    store.put(
        key,
        Page {
            kind,
            resp_hash: site.page_hash(kind, &[]),
            body: Vec::new(),
        },
    );
    PageResult::Ok(text(&key))
}

pub fn rehash<M: Memory>(store: &mut Store<M>, site: &Site) -> u64 {
    let mut changed = 0;
    let mut at = None;
    while let Some((key, mut page)) = store.next_after(at.as_ref()) {
        let resp_hash = site.page_hash(page.kind, &page.body);
        if resp_hash != page.resp_hash {
            page.resp_hash = resp_hash;
            store.put(key, page);
            changed += 1;
        }
        at = Some(key);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http;
    use crate::site;
    use candid::Principal;
    use std::sync::LazyLock;
    use witness_canister_common::Origins;

    static SITE: LazyLock<Site> =
        LazyLock::new(|| Site::new(Origins::default(), &crate::pins::Pins::default()));
    use ic_stable_structures::VectorMemory;
    use witness_format::{
        BlurClass, BootState, CaptureId, CertSerial, Checks, Confidence, Dims, Edit, Geohash5,
        GnssChecks, KeyId, KeyRef, Kind, LocationEvidence, LocationReasons, Rect, SecurityLevel,
        TimeEvidence,
    };

    fn store() -> Store<VectorMemory> {
        Store::init(
            VectorMemory::default(),
            VectorMemory::default(),
            VectorMemory::default(),
        )
    }

    pub(crate) fn derivative(counter: u64) -> WitnessClaim {
        WitnessClaim {
            capture_id: CaptureId::new(0x0002_0304_0506_0708, 1).unwrap(),
            kind: Kind::Derivative {
                parent: RecordId::from_bytes([0x44; 10]),
                root: CaptureId::new(7, 0).unwrap(),
                edits: vec![
                    Edit::Blur {
                        rect: Rect {
                            x: 1,
                            y: 2,
                            w: 3,
                            h: 4,
                        },
                        class: BlurClass::Face,
                        sigma_e2: 300,
                    };
                    2
                ],
            },
            asset_hash: [0xaa; 32],
            pixel_hash: [0xbb; 32],
            fp_commit: [0xcc; 32],
            dims: Dims {
                width: 1080,
                height: 1350,
            },
            time: TimeEvidence {
                wall_ms: 1_700_000_000_000,
                gnss_ms: None,
                elapsed_ns: 5_000_000_000,
                boot_count: 3,
            },
            loc: LocationEvidence {
                commit: None,
                confidence: Confidence::Medium,
                reasons: LocationReasons::FUSED_FIX,
            },
            checks: Checks {
                key_security: SecurityLevel::StrongBox,
                boot: BootState::Verified,
                device_locked: true,
                mock_location: false,
                gnss: GnssChecks::RAW_AVAILABLE,
                recapture: 12,
            },
            key: KeyRef {
                key_id: KeyId::from_bytes([0x22; 16]),
                counter,
            },
            cert_serial: CertSerial::from_u64(0x1234).unwrap(),
            coarse: Geohash5::new("9q8yy"),
        }
    }

    pub(crate) fn input(counter: u64, dev: bool) -> Shell {
        let claim = derivative(counter).to_cbor().unwrap();
        Shell {
            record_id: RecordId::from_claim_cbor(&claim).to_string(),
            record_ix: counter,
            claim,
            media: Principal::from_slice(&[1, 2, 3]),
            media_shard: 2,
            anchored_ns: 1_700_000_100_000_000_000,
            dev,
        }
    }

    fn id_of(i: &Shell) -> String {
        RecordId::from_claim_cbor(&i.claim).to_string()
    }

    #[test]
    fn put_shell_renders_and_certifies() {
        let mut s = store();
        let i = input(1, false);
        let r = put_shell(&mut s, &SITE, &i);
        assert_eq!(r, PageResult::Ok(id_of(&i)));
        let PageResult::Ok(id) = r else {
            unreachable!()
        };
        let resp = site::serve(&SITE, &s, &format!("/p/{id}"), b"c");
        assert_eq!(resp.status_code(), 200);
        let body = String::from_utf8(resp.body().to_vec()).unwrap();
        assert!(body.contains("Matches an original, with 2 declared blurs"));
        assert!(body.contains("near 37.8° N, 122.4° W"));
        assert!(body.contains("14 Nov 2023, 22:13 UTC"));
        assert!(body.contains("14 Nov 2023, 22:15 UTC"));
        assert!(body.contains(&format!(
            "https://{}.icp0.io/o/{id}",
            Principal::from_slice(&[1, 2, 3]).to_text()
        )));
        assert!(body.contains("\"media_shard\":2"));
    }

    #[test]
    fn stored_hash_matches_served_response() {
        let mut s = store();
        for (n, dev) in [(1, false), (2, true)] {
            let i = input(n, dev);
            let PageResult::Ok(id) = put_shell(&mut s, &SITE, &i) else {
                panic!()
            };
            let page = s.get(id.as_bytes().try_into().unwrap()).unwrap();
            let served = site::serve(&SITE, &s, &format!("/p/{id}"), b"c");
            let bare = SITE.response(page.kind.into(), served.body().to_vec());
            assert_eq!(http::hash(&bare), page.resp_hash);
            assert_eq!(
                page.kind,
                if dev {
                    PageKind::DevShell
                } else {
                    PageKind::Shell
                }
            );
        }
    }

    #[test]
    fn put_shell_is_idempotent() {
        let mut s = store();
        let i = input(3, false);
        put_shell(&mut s, &SITE, &i);
        let root = s.root_hash();
        assert_eq!(put_shell(&mut s, &SITE, &i), PageResult::Ok(id_of(&i)));
        assert_eq!(s.root_hash(), root);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn put_shell_rejects_malformed_claims() {
        let mut s = store();
        let mut i = input(4, false);
        i.claim = vec![0xff, 0x00];
        assert!(matches!(
            put_shell(&mut s, &SITE, &i),
            PageResult::Err(PageErr::Malformed(_))
        ));
        i.claim = Vec::new();
        assert!(matches!(
            put_shell(&mut s, &SITE, &i),
            PageResult::Err(PageErr::Malformed(_))
        ));
        i.claim = vec![0u8; Submission::MAX_PART + 1];
        assert_eq!(put_shell(&mut s, &SITE, &i), malformed("claim too large"));
        let mut c = input(4, false).claim;
        c.push(0);
        i.claim = c;
        assert!(matches!(
            put_shell(&mut s, &SITE, &i),
            PageResult::Err(PageErr::Malformed(_))
        ));
        assert!(s.is_empty());
    }

    #[test]
    fn tombstone_is_final_and_recertified() {
        let mut s = store();
        let i = input(5, false);
        let PageResult::Ok(id) = put_shell(&mut s, &SITE, &i) else {
            panic!()
        };
        let before = s.root_hash();
        assert_eq!(
            tombstone(&mut s, &SITE, &id, Removal::Erased),
            PageResult::Ok(id.clone())
        );
        assert_ne!(s.root_hash(), before);
        assert_eq!(put_shell(&mut s, &SITE, &i), PageResult::Err(PageErr::Gone));
        let resp = site::serve(&SITE, &s, &format!("/p/{id}"), b"c");
        assert_eq!(resp.status_code(), 410);
        assert_eq!(resp.body(), SITE.gone(Removal::Erased).as_bytes());
        let after = s.root_hash();
        tombstone(&mut s, &SITE, &id, Removal::Erased);
        assert_eq!(s.root_hash(), after);
        tombstone(&mut s, &SITE, &id, Removal::Takedown);
        assert_ne!(s.root_hash(), after);
    }

    #[test]
    fn tombstone_before_shell_blocks_resurrection() {
        let mut s = store();
        let i = input(6, false);
        let id = id_of(&i);
        assert_eq!(
            tombstone(&mut s, &SITE, &id.to_lowercase(), Removal::Revoked),
            PageResult::Ok(id.clone())
        );
        assert_eq!(put_shell(&mut s, &SITE, &i), PageResult::Err(PageErr::Gone));
    }

    #[test]
    fn tombstone_accepts_grouped_and_rejects_garbage() {
        let mut s = store();
        let id = RecordId::from_bytes([9; 10]);
        assert_eq!(
            tombstone(&mut s, &SITE, &id.grouped(), Removal::Takedown),
            PageResult::Ok(id.to_string())
        );
        for bad in [
            "",
            "short",
            "!!!!!!!!!!!!!!!!",
            "0123456789ABCDEFG",
            "<script>",
        ] {
            assert!(
                matches!(
                    tombstone(&mut s, &SITE, bad, Removal::Erased),
                    PageResult::Err(PageErr::Malformed(_))
                ),
                "{bad}"
            );
        }
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn render_is_deterministic_and_original_copy() {
        let i = input(7, false);
        let c = WitnessClaim::from_cbor(&i.claim).unwrap();
        let id = RecordId::from_claim_cbor(&i.claim);
        assert_eq!(render(&i, &c, &id, &SITE), render(&i, &c, &id, &SITE));
        let mut orig = derivative(7);
        orig.kind = Kind::Original;
        orig.coarse = None;
        orig.loc.commit = Some([1; 32]);
        let h = render(&i, &orig, &id, &SITE);
        assert!(h.contains("<h1>Sealed original</h1>"));
        assert!(!h.contains("<dt>Area</dt>"));
    }

    fn local() -> Site {
        Site::new(
            Origins {
                site: "http://pages-id.localhost:8090".into(),
                case: "http://pages-id.localhost:8090".into(),
                app: "http://web-id.localhost:8090".into(),
                api: "http://localhost:8090".into(),
                canister: "http://*.localhost:8090".into(),
            },
            &crate::pins::Pins::default(),
        )
    }

    #[test]
    fn put_shell_rejects_a_record_id_that_is_not_the_claims() {
        let mut s = store();
        let mut i = input(8, false);
        i.record_id = id_of(&input(9, false));
        assert_eq!(
            put_shell(&mut s, &SITE, &i),
            malformed("record_id does not match claim")
        );
        i.record_id = "garbage".into();
        assert_eq!(
            put_shell(&mut s, &SITE, &i),
            malformed("record_id does not match claim")
        );
        i.record_id = RecordId::from_claim_cbor(&i.claim).grouped().to_lowercase();
        assert_eq!(put_shell(&mut s, &SITE, &i), PageResult::Ok(id_of(&i)));
    }

    #[test]
    fn local_site_serves_shells_that_boot_from_the_local_web_canister() {
        let site = local();
        let mut s = store();
        let i = input(10, true);
        let PageResult::Ok(id) = put_shell(&mut s, &site, &i) else {
            panic!()
        };
        let resp = site::serve(&site, &s, &format!("/p/{id}"), b"c");
        assert_eq!(resp.status_code(), 200);
        let body = String::from_utf8(resp.body().to_vec()).unwrap();
        assert!(body.contains("src=\"http://web-id.localhost:8090/boot.mjs\""));
        assert!(!body.contains("witness.app"));
        let csp = resp
            .headers()
            .iter()
            .find(|(k, _)| k == "content-security-policy")
            .map(|(_, v)| v.clone())
            .unwrap();
        assert_eq!(csp, site.csp.proof);
        let page = s.get(id.as_bytes().try_into().unwrap()).unwrap();
        let bare = site.response(page.kind.into(), resp.body().to_vec());
        assert_eq!(http::hash(&bare), page.resp_hash);
    }

    #[test]
    fn rehash_recertifies_every_page_for_new_origins() {
        let mut s = store();
        for n in 0..5 {
            put_shell(&mut s, &SITE, &input(20 + n, n % 2 == 0));
        }
        let gone = id_of(&input(20, true));
        tombstone(&mut s, &SITE, &gone, Removal::Takedown);
        assert_eq!(rehash(&mut s, &SITE), 0);
        let before = s.root_hash();
        let site = local();
        assert_eq!(rehash(&mut s, &site), 5);
        assert_ne!(s.root_hash(), before);
        assert_eq!(rehash(&mut s, &site), 0);
        let mut at = None;
        while let Some((key, page)) = s.next_after(at.as_ref()) {
            let id: String = key.iter().map(|b| char::from(*b)).collect();
            let served = site::serve(&site, &s, &format!("/p/{id}"), b"c");
            let bare = site.response(page.kind.into(), served.body().to_vec());
            assert_eq!(http::hash(&bare), page.resp_hash, "{id}");
            at = Some(key);
        }
        let served = site::serve(&site, &s, &format!("/p/{gone}"), b"c");
        assert_eq!(served.status_code(), 410);
        assert_eq!(served.body(), site.gone(Removal::Takedown).as_bytes());
        assert_eq!(rehash(&mut s, &SITE), 5);
        assert_eq!(s.root_hash(), before);
    }

    #[test]
    fn rehash_on_empty_store_is_a_no_op() {
        let mut s = store();
        assert_eq!(rehash(&mut s, &local()), 0);
        assert!(s.is_empty());
    }
}
