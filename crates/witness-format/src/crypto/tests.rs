use super::*;
use crate::testutil::Rng;
use alloc::vec::Vec;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;

fn hex(s: &str) -> Vec<u8> {
    let s: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    s.chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

const SPKI_P256_PREFIX: &str = "3059301306072a8648ce3d020106082a8648ce3d030107034200";
const RFC6979_X: &str = "c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721";
const RFC6979_UX: &str = "60fed4ba255a9d31c961eb74c6356d68c049b8923b61fa6ce669622e60f29fb6";
const RFC6979_UY: &str = "7903fe1008b8bc99a41ae9e95628bc64f2f1b20c2d7e9f5177a3c294d4462299";
const SAMPLE_R: &str = "efd48b2aacb6a8fd1140dd9cd45e81d69d2c877b56aaf991c34d0ea84eaf3716";
const SAMPLE_S: &str = "f7cb1c942d657c41d436c7a1b6e29f65f3e900dbb9aff4064dc4ab2f843acda8";
const TEST_R: &str = "f1abb023518351cd71d881567b1ea663ed3efcf6c5132b354f28d3b0b7d38367";
const TEST_S: &str = "019f4113742a2b14bd25926b49c649155f267e60d3814b4c0cc84250e46f0083";
const ORDER: &str = "ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551";

fn sec1() -> Vec<u8> {
    let mut v = hex("04");
    v.extend(hex(RFC6979_UX));
    v.extend(hex(RFC6979_UY));
    v
}

fn spki() -> Vec<u8> {
    let mut v = hex(SPKI_P256_PREFIX);
    v.extend(sec1());
    v
}

fn raw(r: &str, s: &str) -> [u8; 64] {
    let mut v = hex(r);
    v.extend(hex(s));
    v.try_into().unwrap()
}

fn sub_order(s: &[u8]) -> Vec<u8> {
    let n = hex(ORDER);
    let mut out = alloc::vec![0u8; 32];
    let mut borrow = 0i16;
    for i in (0..32).rev() {
        let mut d = i16::from(n[i]) - i16::from(s[i]) - borrow;
        borrow = i16::from(d < 0);
        if d < 0 {
            d += 256;
        }
        out[i] = d as u8;
    }
    out
}

#[test]
fn sha256_golden() {
    assert_eq!(
        sha256(b"").to_vec(),
        hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    assert_eq!(
        sha256(b"abc").to_vec(),
        hex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
    assert_eq!(
        sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").to_vec(),
        hex("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
    );
}

#[test]
fn sha256_parts_equals_concatenation() {
    let mut rng = Rng::new(10);
    for _ in 0..200 {
        let data: [u8; 97] = rng.bytes();
        let a = rng.below(98) as usize;
        let b = a + rng.below((98 - a) as u64) as usize;
        assert_eq!(
            sha256_parts(&[&data[..a], &data[a..b], &data[b..]]),
            sha256(&data)
        );
    }
    assert_eq!(sha256_parts(&[]), sha256(b""));
}

#[test]
fn hash_ds_prefixes_domain() {
    assert_eq!(
        hash_ds(domain::LOCATION, &[b"x"]),
        sha256(b"witness/loc/v1x")
    );
    assert_eq!(hash_ds(domain::ERASE, &[]), sha256(b"witness/erase/v1"));
    assert_ne!(
        hash_ds(domain::LOCATION, &[b"a"]),
        hash_ds(domain::ERASE, &[b"a"])
    );
    assert_eq!(domain::CASE, b"witness/case/v1");
    assert_eq!(domain::OFFLINE_CHALLENGE, b"witness-offline");
}

#[test]
fn ct_eq_semantics() {
    assert!(ct_eq(b"", b""));
    assert!(ct_eq(b"abc", b"abc"));
    assert!(!ct_eq(b"abc", b"abd"));
    assert!(!ct_eq(b"abc", b"ab"));
    assert!(!ct_eq(b"", b"a"));
}

#[test]
fn rfc6979_vectors_verify_with_sec1_and_spki() {
    let sample = raw(SAMPLE_R, SAMPLE_S);
    let test = raw(TEST_R, TEST_S);
    for pk in [sec1(), spki()] {
        assert_eq!(es256_verify(&pk, b"sample", &sample), Ok(()));
        assert_eq!(es256_verify(&pk, b"test", &test), Ok(()));
        assert_eq!(es256_verify(&pk, b"test", &sample), Err(SigError::Invalid));
        assert_eq!(
            es256_verify(&pk, b"Sample", &sample),
            Err(SigError::Invalid)
        );
    }
}

#[test]
fn signing_key_matches_rfc6979_public_key() {
    let sk = SigningKey::from_slice(&hex(RFC6979_X)).unwrap();
    let pk = Es256Key::from_sec1(sk.verifying_key().to_encoded_point(false).as_bytes()).unwrap();
    assert_eq!(pk.sec1_uncompressed().to_vec(), sec1());
    let sig: p256::ecdsa::Signature = sk.sign(b"sample");
    let r: [u8; 64] = sig.to_bytes().into();
    assert_eq!(r, raw(SAMPLE_R, SAMPLE_S));
}

#[test]
fn der_golden_and_roundtrip() {
    let sample = raw(SAMPLE_R, SAMPLE_S);
    let der = es256_raw_to_der(&sample).unwrap();
    let mut want = hex("3046022100");
    want.extend(hex(SAMPLE_R));
    want.extend(hex("022100"));
    want.extend(hex(SAMPLE_S));
    assert_eq!(der.as_bytes(), want.as_slice());
    assert_eq!(es256_der_to_raw(der.as_bytes()), Ok(sample));
    assert_eq!(es256_sig_to_raw(der.as_bytes()), Ok(sample));
    assert_eq!(es256_sig_to_raw(&sample), Ok(sample));
    let test = raw(TEST_R, TEST_S);
    let der = es256_raw_to_der(&test).unwrap();
    assert_eq!(der.as_bytes()[..5], hex("3045022100")[..]);
    assert_eq!(der.as_bytes()[37..39], hex("0220")[..]);
    assert_eq!(es256_verify(&sec1(), b"test", der.as_bytes()), Ok(()));
    assert!(alloc::format!("{der:?}").starts_with("DerSignature"));
}

#[test]
fn der_short_integers_are_minimal() {
    let mut r = [0u8; 32];
    r[31] = 1;
    let mut s = [0u8; 32];
    s[30] = 0x80;
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&r);
    sig[32..].copy_from_slice(&s);
    let der = es256_raw_to_der(&sig).unwrap();
    assert_eq!(der.as_bytes(), hex("3008020101020300 8000").as_slice());
    assert_eq!(es256_der_to_raw(der.as_bytes()), Ok(sig));
}

#[test]
fn der_rejects_malformed() {
    let good = es256_raw_to_der(&raw(TEST_R, TEST_S)).unwrap();
    let g = good.as_bytes();
    let mut trailing = g.to_vec();
    trailing.push(0);
    assert_eq!(es256_der_to_raw(&trailing), Err(SigError::Encoding));
    assert_eq!(es256_der_to_raw(&g[..g.len() - 1]), Err(SigError::Encoding));
    assert_eq!(es256_der_to_raw(&[]), Err(SigError::Encoding));
    let mut padded = hex("304602220000");
    padded.extend(hex(TEST_R));
    padded.extend(hex("0220"));
    padded.extend(hex(TEST_S));
    assert_eq!(es256_der_to_raw(&padded), Err(SigError::Encoding));
    let mut wrong_tag = g.to_vec();
    wrong_tag[0] = 0x31;
    assert_eq!(es256_der_to_raw(&wrong_tag), Err(SigError::Encoding));
    assert_eq!(
        es256_der_to_raw(&hex("3006020100020101")),
        Err(SigError::Encoding)
    );
    assert_eq!(
        es256_der_to_raw(&hex("3006020101020180")),
        Err(SigError::Encoding)
    );
}

#[test]
fn raw_rejects_out_of_range_scalars() {
    let zero_r = raw(&"00".repeat(32), TEST_S);
    assert_eq!(es256_raw_to_der(&zero_r), Err(SigError::Encoding));
    assert_eq!(
        es256_verify(&sec1(), b"test", &zero_r),
        Err(SigError::Encoding)
    );
    let order_s = raw(TEST_R, ORDER);
    assert_eq!(es256_sig_to_raw(&order_s), Err(SigError::Encoding));
    let max = raw(TEST_R, &"ff".repeat(32));
    assert_eq!(es256_sig_to_raw(&max), Err(SigError::Encoding));
}

#[test]
fn wrong_signature_lengths_are_rejected() {
    let sample = raw(SAMPLE_R, SAMPLE_S);
    assert_eq!(
        es256_verify(&sec1(), b"sample", &sample[..63]),
        Err(SigError::Encoding)
    );
    let mut long = sample.to_vec();
    long.push(0);
    assert_eq!(
        es256_verify(&sec1(), b"sample", &long),
        Err(SigError::Encoding)
    );
    assert_eq!(
        es256_verify(&sec1(), b"sample", &[]),
        Err(SigError::Encoding)
    );
}

#[test]
fn low_s_detection_and_normalization() {
    let sample = raw(SAMPLE_R, SAMPLE_S);
    assert_eq!(es256_is_low_s(&sample), Ok(false));
    let low = es256_normalize_low_s(&sample).unwrap();
    assert_eq!(low[..32], sample[..32]);
    assert_eq!(low[32..].to_vec(), sub_order(&hex(SAMPLE_S)));
    assert_eq!(es256_is_low_s(&low), Ok(true));
    assert_eq!(es256_normalize_low_s(&low), Ok(low));
    assert_eq!(es256_verify(&sec1(), b"sample", &low), Ok(()));
    assert_eq!(es256_verify(&sec1(), b"sample", &sample), Ok(()));
    assert_eq!(es256_verify_low_s(&sec1(), b"sample", &low), Ok(()));
    assert_eq!(
        es256_verify_low_s(&sec1(), b"sample", &sample),
        Err(SigError::HighS)
    );
    let der_high = es256_raw_to_der(&sample).unwrap();
    assert_eq!(es256_is_low_s(der_high.as_bytes()), Ok(false));
    assert_eq!(es256_normalize_low_s(der_high.as_bytes()), Ok(low));
    let test = raw(TEST_R, TEST_S);
    assert_eq!(es256_is_low_s(&test), Ok(true));
    assert_eq!(es256_normalize_low_s(&test), Ok(test));
}

#[test]
fn public_key_parsing() {
    assert!(Es256Key::parse(&sec1()).is_ok());
    assert!(Es256Key::parse(&spki()).is_ok());
    let compressed = {
        let mut c = alloc::vec![if hex(RFC6979_UY)[31] & 1 == 0 { 2 } else { 3 }];
        c.extend(hex(RFC6979_UX));
        c
    };
    let ck = Es256Key::parse(&compressed).unwrap();
    assert_eq!(ck.sec1_uncompressed().to_vec(), sec1());
    assert_eq!(Es256Key::parse(&[]), Err(SigError::Key));
    assert_eq!(Es256Key::parse(&[4; 65]), Err(SigError::Key));
    let mut off_curve = sec1();
    off_curve[64] ^= 1;
    assert_eq!(Es256Key::parse(&off_curve), Err(SigError::Key));
    let mut bad_spki = spki();
    bad_spki[10] ^= 0xff;
    assert_eq!(Es256Key::parse(&bad_spki), Err(SigError::Key));
    assert_eq!(Es256Key::parse(&spki()[..90]), Err(SigError::Key));
    let mut spki_trailing = spki();
    spki_trailing.push(0);
    assert_eq!(Es256Key::from_spki(&spki_trailing), Err(SigError::Key));
    assert_eq!(Es256Key::from_sec1(&spki()), Err(SigError::Key));
    assert_eq!(es256_verify(&[0x30], b"", &[0; 64]), Err(SigError::Key));
}

#[test]
fn verify_prehash_matches_verify() {
    let pk = Es256Key::parse(&spki()).unwrap();
    let sample = raw(SAMPLE_R, SAMPLE_S);
    assert_eq!(pk.verify_prehash(&sha256(b"sample"), &sample), Ok(()));
    assert_eq!(
        pk.verify_prehash(&sha256(b"other"), &sample),
        Err(SigError::Invalid)
    );
}

#[test]
fn property_sign_verify_seeded() {
    let mut rng = Rng::new(11);
    for _ in 0..24 {
        let secret: [u8; 32] = rng.bytes();
        let Ok(sk) = SigningKey::from_slice(&secret) else {
            continue;
        };
        let pk_bytes = sk.verifying_key().to_encoded_point(false);
        let pk = Es256Key::from_sec1(pk_bytes.as_bytes()).unwrap();
        let mut msg = alloc::vec![0u8; rng.below(200) as usize];
        rng.fill(&mut msg);
        let sig: p256::ecdsa::Signature = sk.sign(&msg);
        let raw: [u8; 64] = sig.to_bytes().into();
        let der = es256_raw_to_der(&raw).unwrap();
        assert_eq!(pk.verify(&msg, &raw), Ok(()));
        assert_eq!(pk.verify(&msg, der.as_bytes()), Ok(()));
        let low = es256_normalize_low_s(&raw).unwrap();
        assert_eq!(pk.verify_low_s(&msg, &low), Ok(()));
        let mut flipped = raw;
        let at = rng.below(64) as usize;
        flipped[at] ^= 1 << rng.below(8);
        assert!(pk.verify(&msg, &flipped).is_err());
        msg.push(1);
        assert_eq!(pk.verify(&msg, &raw), Err(SigError::Invalid));
    }
}

const ED_PK1: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const ED_SIG1: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
const ED_PK2: &str = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
const ED_SIG2: &str = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

#[test]
fn ed25519_rfc8032_vectors() {
    assert_eq!(ed25519_verify(&hex(ED_PK1), b"", &hex(ED_SIG1)), Ok(()));
    assert_eq!(ed25519_verify(&hex(ED_PK2), &[0x72], &hex(ED_SIG2)), Ok(()));
    assert_eq!(
        ed25519_verify(&hex(ED_PK1), &[0x72], &hex(ED_SIG1)),
        Err(SigError::Invalid)
    );
    assert_eq!(
        ed25519_verify(&hex(ED_PK2), &[0x72], &hex(ED_SIG1)),
        Err(SigError::Invalid)
    );
}

#[test]
fn ed25519_rejects_malformed() {
    let sig = hex(ED_SIG1);
    let pk = hex(ED_PK1);
    assert_eq!(ed25519_verify(&pk[..31], b"", &sig), Err(SigError::Key));
    assert_eq!(
        ed25519_verify(&pk, b"", &sig[..63]),
        Err(SigError::Encoding)
    );
    let mut s_high = sig.clone();
    s_high[63] |= 0xf0;
    assert_eq!(ed25519_verify(&pk, b"", &s_high), Err(SigError::Invalid));
    let mut identity = [0u8; 32];
    identity[0] = 1;
    assert!(ed25519_verify(&identity, b"", &sig).is_err());
    let mut flipped = sig.clone();
    flipped[5] ^= 4;
    assert_eq!(ed25519_verify(&pk, b"", &flipped), Err(SigError::Invalid));
}

#[test]
fn ed25519_seeded_sign_verify() {
    let mut rng = Rng::new(12);
    for _ in 0..32 {
        let sk = ed25519_dalek::SigningKey::from_bytes(&rng.bytes());
        let mut msg = alloc::vec![0u8; rng.below(300) as usize];
        rng.fill(&mut msg);
        use ed25519_dalek::Signer as _;
        let sig = sk.sign(&msg).to_bytes();
        let pk = sk.verifying_key().to_bytes();
        assert_eq!(ed25519_verify(&pk, &msg, &sig), Ok(()));
        msg.push(0);
        assert_eq!(ed25519_verify(&pk, &msg, &sig), Err(SigError::Invalid));
    }
}

#[test]
fn sig_error_display() {
    for e in [
        SigError::Key,
        SigError::Encoding,
        SigError::HighS,
        SigError::Invalid,
    ] {
        assert!(!alloc::format!("{e}").is_empty());
    }
}
