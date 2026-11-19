use super::*;
use crate::assertions::cbor_assertion_box;
use crate::claim::LABEL_V1;
use crate::jumbf::{JSON, write_box, write_superbox};
use crate::testkit::Rng;
use alloc::vec;

const LABEL: &str = "urn:c2pa:test";

fn store(labels: &[&str]) -> Vec<u8> {
    let boxes: Vec<Vec<u8>> = labels
        .iter()
        .map(|l| cbor_assertion_box(l, &[0xa0]).unwrap())
        .collect();
    let refs: Vec<&[u8]> = boxes.iter().map(Vec::as_slice).collect();
    let mut out = Vec::new();
    write_store(&mut out, LABEL, &refs, b"claim", b"cose").unwrap();
    out
}

fn custom_manifest(body: impl FnOnce(&mut Vec<u8>) -> Result<(), Error>) -> Vec<u8> {
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, STORE_LABEL, |o| {
        write_superbox(o, &uuid::MANIFEST, LABEL, body)?;
        Ok(())
    })
    .unwrap();
    out
}

fn part(o: &mut Vec<u8>, u: &jumbf::Uuid, label: &str, content: &[u8]) -> Result<(), Error> {
    write_superbox(o, u, label, |o| write_box(o, CBOR, content)).map(drop)
}

fn assertions(o: &mut Vec<u8>, labels: &[&str]) -> Result<(), Error> {
    write_superbox(o, &uuid::ASSERTIONS, ASSERTIONS_LABEL, |o| {
        for l in labels {
            o.extend(cbor_assertion_box(l, &[0xa0])?);
        }
        Ok(())
    })
    .map(drop)
}

#[test]
fn store_round_trips() {
    let s = store(&["c2pa.actions.v2", "c2pa.hash.data", "app.witness.claim"]);
    let m = parse_manifest(&active_manifest(&s).unwrap()).unwrap();
    assert_eq!(m.label, LABEL);
    assert_eq!(m.claim_label, LABEL_V2);
    assert_eq!(m.claim, b"claim");
    assert_eq!(m.signature, b"cose");
    let labels: Vec<_> = m.assertions.iter().map(|a| a.label).collect();
    assert_eq!(
        labels,
        vec!["app.witness.claim", "c2pa.actions.v2", "c2pa.hash.data"]
    );
    assert!(m.has_assertion("c2pa.hash.data"));
    assert!(!m.has_assertion("c2pa.hash.bmff"));
    assert_eq!(
        m.assertion("app.witness.claim").unwrap().cbor(),
        Ok(&[0xa0][..])
    );
}

#[test]
fn assertion_payload_is_superbox_payload() {
    let s = store(&["a"]);
    let m = parse_manifest(&active_manifest(&s).unwrap()).unwrap();
    let b = cbor_assertion_box("a", &[0xa0]).unwrap();
    assert_eq!(m.assertions[0].payload, &b[8..]);
}

#[test]
fn last_manifest_is_active() {
    let first = store(&["a"]);
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, STORE_LABEL, |o| {
        let inner = SuperBox::from_bytes(&first).unwrap().content;
        o.extend_from_slice(inner);
        write_superbox(o, &uuid::MANIFEST, "urn:c2pa:second", |o| {
            assertions(o, &[])
        })?;
        Ok(())
    })
    .unwrap();
    assert_eq!(store_manifests(&out).unwrap().len(), 2);
    assert_eq!(
        active_manifest(&out).unwrap().label(),
        Some("urn:c2pa:second")
    );
}

#[test]
fn store_identity_is_checked() {
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::MANIFEST, STORE_LABEL, |_| Ok(())).unwrap();
    assert_eq!(active_manifest(&out), Err(Error::MalformedJumbf));
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, "c2pX", |_| Ok(())).unwrap();
    assert_eq!(active_manifest(&out), Err(Error::MalformedJumbf));
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, STORE_LABEL, |_| Ok(())).unwrap();
    assert_eq!(
        active_manifest(&out),
        Err(Error::MissingPart(Part::Manifest))
    );
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, STORE_LABEL, |o| {
        write_box(o, CBOR, &[1])
    })
    .unwrap();
    assert_eq!(active_manifest(&out), Err(Error::MalformedJumbf));
}

#[test]
fn missing_parts_are_reported() {
    let cases: [(Vec<u8>, Part); 3] = [
        (
            custom_manifest(|o| {
                part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
                part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
            }),
            Part::AssertionStore,
        ),
        (
            custom_manifest(|o| {
                assertions(o, &["a"])?;
                part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
            }),
            Part::Claim,
        ),
        (
            custom_manifest(|o| {
                assertions(o, &["a"])?;
                part(o, &uuid::CLAIM, LABEL_V2, b"c")
            }),
            Part::Signature,
        ),
    ];
    for (bytes, p) in cases {
        assert_eq!(
            parse_manifest(&active_manifest(&bytes).unwrap()),
            Err(Error::MissingPart(p))
        );
    }
}

#[test]
fn duplicate_or_mislabeled_parts_are_rejected() {
    let dup_claim = custom_manifest(|o| {
        assertions(o, &["a"])?;
        part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
        part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
        part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
    });
    let wrong_sig_label = custom_manifest(|o| {
        assertions(o, &["a"])?;
        part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
        part(o, &uuid::SIGNATURE, "c2pa.sig", b"s")
    });
    let dup_assertion = custom_manifest(|o| {
        assertions(o, &["a", "b", "a"])?;
        part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
        part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
    });
    let two_claim_boxes = custom_manifest(|o| {
        assertions(o, &["a"])?;
        write_superbox(o, &uuid::CLAIM, LABEL_V2, |o| {
            write_box(o, CBOR, b"c")?;
            write_box(o, CBOR, b"d")
        })?;
        part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
    });
    for (bytes, e) in [
        (dup_claim, Error::MalformedJumbf),
        (wrong_sig_label, Error::MalformedJumbf),
        (dup_assertion, Error::DuplicateAssertion),
        (two_claim_boxes, Error::MalformedJumbf),
    ] {
        assert_eq!(parse_manifest(&active_manifest(&bytes).unwrap()), Err(e));
    }
}

#[test]
fn unknown_children_are_ignored_and_v1_label_surfaces() {
    let bytes = custom_manifest(|o| {
        assertions(o, &["a"])?;
        part(o, &uuid::CLAIM, LABEL_V1, b"c")?;
        write_superbox(
            o,
            &crate::jumbf::iso_uuid(*b"c2vc"),
            "c2pa.credentials",
            |_| Ok(()),
        )?;
        part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
    });
    let m = parse_manifest(&active_manifest(&bytes).unwrap()).unwrap();
    assert_eq!(m.claim_label, LABEL_V1);
}

#[test]
fn non_manifest_uuid_is_rejected() {
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, STORE_LABEL, |o| {
        write_superbox(o, &uuid::UPDATE_MANIFEST, LABEL, |_| Ok(()))?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        parse_manifest(&active_manifest(&out).unwrap()),
        Err(Error::MalformedJumbf)
    );
}

#[test]
fn assertion_content_must_be_single_cbor_box() {
    let bytes = custom_manifest(|o| {
        write_superbox(o, &uuid::ASSERTIONS, ASSERTIONS_LABEL, |o| {
            write_superbox(o, &uuid::JSON, "j", |o| write_box(o, JSON, b"{}"))?;
            write_superbox(o, &uuid::CBOR, "k", |o| write_box(o, JSON, b"{}"))?;
            Ok(())
        })?;
        part(o, &uuid::CLAIM, LABEL_V2, b"c")?;
        part(o, &uuid::SIGNATURE, SIGNATURE_LABEL, b"s")
    });
    let m = parse_manifest(&active_manifest(&bytes).unwrap()).unwrap();
    assert_eq!(
        m.assertion("j").unwrap().cbor(),
        Err(Error::MalformedAssertion)
    );
    assert_eq!(
        m.assertion("k").unwrap().cbor(),
        Err(Error::MalformedAssertion)
    );
}

#[test]
fn random_mutations_never_panic() {
    let base = store(&["c2pa.actions.v2", "c2pa.hash.data", "app.witness.claim"]);
    let mut rng = Rng::new(1);
    for _ in 0..3000 {
        let mut d = base.clone();
        let i = rng.below(d.len());
        d[i] = rng.next() as u8;
        if let Ok(a) = active_manifest(&d)
            && let Ok(m) = parse_manifest(&a)
        {
            m.assertions.iter().for_each(|x| {
                let _ = x.cbor();
            });
        }
    }
}
