use super::*;
use crate::testkit::{Rng, device_key, spki};
use alloc::vec;
use p256::ecdsa::signature::Signer as _;

fn enc(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
    let mut v = Vec::new();
    f(&mut Encoder::new(&mut v));
    v
}

fn key() -> Es256Key {
    Es256Key::from_spki(&spki(&device_key())).unwrap()
}

fn signed(payload: &[u8], chain: &[Vec<u8>]) -> Vec<u8> {
    let protected = protected_header(chain);
    let sig: p256::ecdsa::Signature = device_key().sign(&sig_structure(&protected, payload));
    encode_sign1(&protected, &sig.to_bytes().into())
}

fn sign1_with(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
    enc(|e| {
        e.head(TAG_MAJOR, TAG_SIGN1);
        f(e);
    })
}

fn protected_with(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
    let p = enc(f);
    sign1_with(|e| {
        e.array(4);
        e.bytes(&p);
        e.map(0);
        e.null();
        e.bytes(&[0; 64]);
    })
}

#[test]
fn protected_header_encodes_alg_and_chain() {
    let one = protected_header(&[vec![1, 2]]);
    assert_eq!(one, vec![0xa2, 0x01, 0x26, 0x18, 0x21, 0x42, 1, 2]);
    let two = protected_header(&[vec![1], vec![2, 3]]);
    assert_eq!(
        two,
        vec![0xa2, 0x01, 0x26, 0x18, 0x21, 0x82, 0x41, 1, 0x42, 2, 3]
    );
}

#[test]
fn sig_structure_matches_rfc9052_layout() {
    let s = sig_structure(&[0xa0], b"pl");
    let mut expected = vec![0x84, 0x6a];
    expected.extend_from_slice(b"Signature1");
    expected.extend_from_slice(&[0x41, 0xa0, 0x40, 0x42, b'p', b'l']);
    assert_eq!(s, expected);
}

#[test]
fn sign1_round_trips() {
    for chain in [vec![vec![9; 300]], vec![vec![7; 10], vec![8; 20]]] {
        let bytes = signed(b"claim", &chain);
        assert_eq!(bytes[0], 0xd2);
        let s = Sign1::parse(&bytes).unwrap();
        assert_eq!(s.alg, ALG_ES256);
        assert_eq!(
            s.x5chain,
            chain.iter().map(Vec::as_slice).collect::<Vec<_>>()
        );
        assert_eq!(s.signature.len(), 64);
        assert_eq!(s.verify(b"claim", &key()), Ok(()));
    }
}

#[test]
fn verify_detects_tampering() {
    let bytes = signed(b"claim", &[vec![1]]);
    let s = Sign1::parse(&bytes).unwrap();
    assert_eq!(s.verify(b"claim!", &key()), Err(Error::Signature));
    let other = Es256Key::from_spki(&spki(
        &p256::ecdsa::SigningKey::from_slice(&[3; 32]).unwrap(),
    ))
    .unwrap();
    assert_eq!(s.verify(b"claim", &other), Err(Error::Signature));
    let mut t = bytes.clone();
    let last = t.len() - 1;
    t[last] ^= 1;
    assert_eq!(
        Sign1::parse(&t).unwrap().verify(b"claim", &key()),
        Err(Error::Signature)
    );
}

#[test]
fn verify_requires_es256_and_raw_signature() {
    let bytes = signed(b"c", &[vec![1]]);
    let mut s = Sign1::parse(&bytes).unwrap();
    s.alg = -35;
    assert_eq!(s.verify(b"c", &key()), Err(Error::UnsupportedAlgorithm));
    let mut s = Sign1::parse(&bytes).unwrap();
    let der = p256::ecdsa::Signature::from_slice(s.signature)
        .unwrap()
        .to_der();
    let der_bytes = der.as_bytes().to_vec();
    s.signature = &der_bytes;
    assert_eq!(s.verify(b"c", &key()), Err(Error::Signature));
}

#[test]
fn unprotected_header_contents_are_ignored() {
    let p = protected_header(&[vec![1]]);
    let bytes = sign1_with(|e| {
        e.array(4);
        e.bytes(&p);
        e.map(2);
        e.text("pad");
        e.bytes(&[0; 10]);
        e.i64(33);
        e.bytes(&[5]);
        e.null();
        e.bytes(&[0; 64]);
    });
    assert_eq!(Sign1::parse(&bytes).unwrap().x5chain, vec![&[1][..]]);
}

#[test]
fn chain_only_in_unprotected_is_missing() {
    let p = enc(|e| {
        e.map(1);
        e.i64(HDR_ALG);
        e.i64(ALG_ES256);
    });
    let bytes = sign1_with(|e| {
        e.array(4);
        e.bytes(&p);
        e.map(1);
        e.i64(HDR_X5CHAIN);
        e.bytes(&[1]);
        e.null();
        e.bytes(&[0; 64]);
    });
    assert_eq!(Sign1::parse(&bytes), Err(Error::MissingPart(Part::X5Chain)));
}

#[test]
fn malformed_envelopes_are_rejected() {
    let p = protected_header(&[vec![1]]);
    let cases = [
        enc(|e| {
            e.array(4);
            e.bytes(&p);
            e.map(0);
            e.null();
            e.bytes(&[0; 64]);
        }),
        enc(|e| {
            e.head(TAG_MAJOR, 98);
            e.array(4);
            e.bytes(&p);
            e.map(0);
            e.null();
            e.bytes(&[0; 64]);
        }),
        sign1_with(|e| {
            e.array(3);
            e.bytes(&p);
            e.map(0);
            e.null();
        }),
        sign1_with(|e| {
            e.array(4);
            e.bytes(&p);
            e.map(0);
            e.bytes(b"attached");
            e.bytes(&[0; 64]);
        }),
        sign1_with(|e| {
            e.array(4);
            e.bytes(&p);
            e.array(0);
            e.null();
            e.bytes(&[0; 64]);
        }),
        sign1_with(|e| {
            e.array(4);
            e.map(0);
            e.map(0);
            e.null();
            e.bytes(&[0; 64]);
        }),
        sign1_with(|e| {
            e.array(4);
            e.bytes(&p);
            e.map(0);
            e.null();
            e.bytes(&[0; 64]);
            e.null();
        }),
        vec![],
        vec![0xd2],
    ];
    for (i, c) in cases.iter().enumerate() {
        assert!(Sign1::parse(c).is_err(), "case {i}");
    }
}

#[test]
fn malformed_protected_headers_are_rejected() {
    let cases: [Vec<u8>; 7] = [
        protected_with(|e| {
            e.map(1);
            e.i64(HDR_X5CHAIN);
            e.bytes(&[1]);
        }),
        protected_with(|e| {
            e.map(3);
            e.i64(HDR_ALG);
            e.i64(ALG_ES256);
            e.i64(HDR_X5CHAIN);
            e.bytes(&[1]);
            e.i64(HDR_ALG);
            e.i64(ALG_ES256);
        }),
        protected_with(|e| {
            e.map(3);
            e.i64(HDR_ALG);
            e.i64(ALG_ES256);
            e.i64(HDR_CRIT);
            e.array(1);
            e.i64(99);
            e.i64(HDR_X5CHAIN);
            e.bytes(&[1]);
        }),
        protected_with(|e| {
            e.map(2);
            e.i64(HDR_ALG);
            e.i64(ALG_ES256);
            e.i64(HDR_X5CHAIN);
            e.array(0);
        }),
        protected_with(|e| {
            e.map(2);
            e.i64(HDR_ALG);
            e.i64(ALG_ES256);
            e.i64(HDR_X5CHAIN);
            e.array(9);
            (0..9).for_each(|_| e.bytes(&[1]));
        }),
        protected_with(|e| {
            e.map(2);
            e.i64(HDR_ALG);
            e.text("ES256");
            e.i64(HDR_X5CHAIN);
            e.bytes(&[1]);
        }),
        protected_with(|e| {
            e.map(2);
            e.text("alg");
            e.i64(ALG_ES256);
            e.i64(HDR_X5CHAIN);
            e.bytes(&[1]);
        }),
    ];
    for (i, c) in cases.iter().enumerate() {
        assert!(Sign1::parse(c).is_err(), "case {i}");
    }
}

#[test]
fn unknown_protected_parameters_are_tolerated() {
    let bytes = protected_with(|e| {
        e.map(3);
        e.i64(HDR_ALG);
        e.i64(ALG_ES256);
        e.i64(3);
        e.text("application/c2pa");
        e.i64(HDR_X5CHAIN);
        e.bytes(&[1]);
    });
    assert!(Sign1::parse(&bytes).is_ok());
}

#[test]
fn random_mutations_never_panic() {
    let base = signed(b"payload", &[vec![1; 40], vec![2; 40]]);
    let mut rng = Rng::new(1);
    for _ in 0..3000 {
        let mut d = base.clone();
        let i = rng.below(d.len());
        d[i] = rng.next() as u8;
        if let Ok(s) = Sign1::parse(&d) {
            let _ = s.verify(b"payload", &key());
        }
    }
}
