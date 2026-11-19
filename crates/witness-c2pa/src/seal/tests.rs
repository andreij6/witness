use super::*;
use crate::app11::{MIN_CHUNK, locate, packetized_len};
use crate::assertions::DataHash;
use crate::manifest::{active_manifest, parse_manifest};
use crate::testkit::*;
use crate::verify::verify_jpeg;
use crate::{ChainError, Credentials, jpeg};
use alloc::vec;

fn seal_with(j: &[u8], signer: &dyn Signer, chunk: usize) -> Result<Sealed, Error> {
    let claim = claim_cbor_for(j);
    seal_jpeg_chunked(j, &spec(&claim), signer, &anchor(), chunk)
}

fn declared_exclusion(out: &[u8]) -> Exclusion {
    let l = locate(out).unwrap();
    let store = l.store.unwrap();
    let m = parse_manifest(&active_manifest(&store).unwrap()).unwrap();
    let dh = DataHash::parse(m.assertion(DATA_HASH).unwrap().cbor().unwrap()).unwrap();
    assert_eq!(dh.exclusions.len(), 1);
    dh.exclusions[0]
}

#[test]
fn seal_preserves_every_original_byte() {
    let j = jpeg(1);
    let s = sealed(&j);
    let l = locate(&s.jpeg).unwrap();
    let mut kept = Vec::new();
    let mut at = 0;
    for r in &l.excluded {
        kept.extend_from_slice(&s.jpeg[at..r.start]);
        at = r.end;
    }
    kept.extend_from_slice(&s.jpeg[at..]);
    assert_eq!(kept, j);
    assert_eq!(s.data_hash, crate::data_hash(&j).unwrap());
    assert_eq!(s.data_hash, crate::data_hash(&s.jpeg).unwrap());
}

#[test]
fn manifest_goes_right_after_app0() {
    let j = jpeg(2);
    let s = sealed(&j);
    let markers: Vec<u8> = jpeg::segments(&s.jpeg)
        .unwrap()
        .map(|x| x.unwrap().marker)
        .collect();
    assert_eq!(&markers[..2], &[0xe0, 0xeb]);
    assert_eq!(
        locate(&s.jpeg).unwrap().excluded[0].start,
        locate(&j).unwrap().insert_at
    );
}

#[test]
fn manifest_goes_after_exif() {
    let mut rng = Rng::new(3);
    let j = jpeg_with(&mut rng, &[(0xe1, b"Exif\0\0data".to_vec())], 128);
    let s = sealed(&j);
    let markers: Vec<u8> = jpeg::segments(&s.jpeg)
        .unwrap()
        .map(|x| x.unwrap().marker)
        .collect();
    assert_eq!(&markers[..3], &[0xe0, 0xe1, 0xeb]);
}

#[test]
fn declared_exclusion_matches_actual_segments() {
    let j = jpeg(4);
    for chunk in [MIN_CHUNK, 50, 333, 1024, MAX_CHUNK] {
        let s = seal_with(&j, &TestSigner::new(true), chunk).unwrap();
        let l = locate(&s.jpeg).unwrap();
        let total: usize = l.excluded.iter().map(|r| r.len()).sum();
        let e = declared_exclusion(&s.jpeg);
        assert_eq!(e.start as usize, l.excluded[0].start, "chunk {chunk}");
        assert_eq!(e.length as usize, total, "chunk {chunk}");
        assert_eq!(total, packetized_len(l.store.unwrap().len(), chunk));
        for w in l.excluded.windows(2) {
            assert_eq!(w[0].end, w[1].start);
        }
    }
}

#[test]
fn der_and_raw_signers_produce_identical_files() {
    let j = jpeg(5);
    let der = seal_with(&j, &TestSigner::new(true), MAX_CHUNK).unwrap();
    let raw = seal_with(&j, &TestSigner::new(false), MAX_CHUNK).unwrap();
    assert_eq!(der, raw);
}

#[test]
fn sealing_is_deterministic() {
    let j = jpeg(6);
    assert_eq!(sealed(&j), sealed(&j));
}

#[test]
fn resealing_replaces_the_existing_store() {
    let j = jpeg(7);
    let once = sealed(&j);
    let twice = sealed(&once.jpeg);
    assert_eq!(once.data_hash, twice.data_hash);
    assert_eq!(locate(&twice.jpeg).unwrap().excluded.len(), 1);
    assert_eq!(once.jpeg, twice.jpeg);
}

#[test]
fn foreign_jumbf_keeps_its_instance_number() {
    let mut contents = b"JP".to_vec();
    contents.extend_from_slice(&app11::DEFAULT_EN.to_be_bytes());
    contents.extend_from_slice(&1u32.to_be_bytes());
    contents.extend_from_slice(b"foreign jumbf payload");
    let mut rng = Rng::new(8);
    let j = jpeg_with(&mut rng, &[(0xeb, contents)], 128);
    let s = sealed(&j);
    let l = locate(&s.jpeg).unwrap();
    assert_eq!(l.foreign_en, vec![app11::DEFAULT_EN]);
    let ours = app11::packet(&s.jpeg[l.excluded[0].start + 4..l.excluded[0].end]).unwrap();
    assert_ne!(ours.en, app11::DEFAULT_EN);
    assert!(
        verify_jpeg(&s.jpeg, &anchor(), Some(NOW))
            .unwrap()
            .binding_intact()
    );
}

#[test]
fn claim_must_bind_the_asset() {
    let j = jpeg(9);
    let wrong = claim_cbor_for(&jpeg(10));
    assert_eq!(
        seal_jpeg(&j, &spec(&wrong), &TestSigner::new(true), &anchor()),
        Err(Error::AssetHashMismatch)
    );
}

#[test]
fn claim_must_name_the_signing_key() {
    let j = jpeg(11);
    let h = crate::data_hash(&j).unwrap();
    let other = P256Key::from_slice(&[0x43; 32]).unwrap();
    let claim = claim_for(h, &spki(&other), LEAF_SERIAL).to_cbor().unwrap();
    assert_eq!(
        seal_jpeg(&j, &spec(&claim), &TestSigner::new(true), &anchor()),
        Err(Error::KeyMismatch)
    );
    let claim = claim_for(h, &spki(&device_key()), LEAF_SERIAL + 1)
        .to_cbor()
        .unwrap();
    assert_eq!(
        seal_jpeg(&j, &spec(&claim), &TestSigner::new(true), &anchor()),
        Err(Error::SerialMismatch)
    );
}

use p256::ecdsa::SigningKey as P256Key;

#[test]
fn signer_with_wrong_key_is_rejected() {
    let mut signer = TestSigner::new(true);
    signer.key = P256Key::from_slice(&[0x44; 32]).unwrap();
    assert_eq!(seal_with(&jpeg(12), &signer, MAX_CHUNK), Err(Error::Signer));
}

struct Garbage(Vec<u8>);

impl Signer for Garbage {
    fn sign(&self, _: &[u8]) -> Vec<u8> {
        self.0.clone()
    }

    fn cert_chain(&self) -> Vec<Vec<u8>> {
        TestSigner::new(true).chain
    }
}

#[test]
fn garbage_signatures_are_rejected() {
    for g in [
        vec![],
        vec![0; 63],
        vec![0; 64],
        vec![0x30, 0x02, 0x01],
        vec![0xff; 72],
    ] {
        assert_eq!(
            seal_with(&jpeg(13), &Garbage(g), MAX_CHUNK),
            Err(Error::Signer)
        );
    }
}

#[test]
fn chain_must_lead_to_anchor() {
    let mut signer = TestSigner::new(true);
    signer.chain = vec![];
    assert_eq!(
        seal_with(&jpeg(14), &signer, MAX_CHUNK),
        Err(Error::Chain(ChainError::Length))
    );
    signer.chain = vec![leaf_der(), root_der_with(&other_root_key())];
    assert_eq!(
        seal_with(&jpeg(14), &signer, MAX_CHUNK),
        Err(Error::Chain(ChainError::Anchor))
    );
    signer.chain = vec![root_der()];
    assert!(matches!(
        seal_with(&jpeg(14), &signer, MAX_CHUNK),
        Err(Error::Chain(_))
    ));
}

#[test]
fn single_certificate_chain_is_supported() {
    let mut signer = TestSigner::new(true);
    signer.chain = vec![leaf_der()];
    let s = seal_with(&jpeg(15), &signer, MAX_CHUNK).unwrap();
    assert!(
        verify_jpeg(&s.jpeg, &anchor(), Some(NOW))
            .unwrap()
            .binding_intact()
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    let j = jpeg(16);
    assert_eq!(
        seal_jpeg(b"not a jpeg", &spec(&[]), &TestSigner::new(true), &anchor()),
        Err(Error::NotJpeg)
    );
    assert!(matches!(
        seal_jpeg(&j, &spec(&[0xa0]), &TestSigner::new(true), &anchor()),
        Err(Error::WitnessClaim(_))
    ));
    let claim = claim_cbor_for(&j);
    let mut sp = spec(&claim);
    sp.actions = &[];
    assert_eq!(
        seal_jpeg(&j, &sp, &TestSigner::new(true), &anchor()),
        Err(Error::MalformedAssertion)
    );
    assert_eq!(
        seal_with(&j, &TestSigner::new(true), MIN_CHUNK - 1),
        Err(Error::TooLarge)
    );
}

#[test]
fn many_random_files_seal_and_verify() {
    let mut rng = Rng::new(17);
    for i in 0..24 {
        let exif_len = rng.below(500);
        let extra = if rng.below(2) == 0 {
            vec![(0xe1, rng.bytes(exif_len))]
        } else {
            vec![]
        };
        let entropy = 16 + rng.below(4000);
        let j = jpeg_with(&mut rng, &extra, entropy);
        let chunk = MIN_CHUNK + rng.below(3000);
        let s = seal_with(&j, &TestSigner::new(i % 2 == 0), chunk).unwrap();
        let r = verify_jpeg(&s.jpeg, &anchor(), Some(NOW)).unwrap();
        assert!(matches!(r.credentials, Credentials::Witness(_)));
        assert!(r.binding_intact(), "file {i}");
    }
}
