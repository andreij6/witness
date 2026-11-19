use crate::attest;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer as _;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use witness_android::{Enrolled, Enrollment, KeystoreSigner, WitnessError};
use witness_core::c2pa::TrustAnchor;
use witness_core::c2pa::x509::verify_chain;
use witness_verdict::testkit::TestDevice;

pub struct SmokeSigner(pub SigningKey);

impl KeystoreSigner for SmokeSigner {
    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, WitnessError> {
        let sig: p256::ecdsa::Signature = self.0.sign(&message);
        Ok(sig.to_der().as_bytes().to_vec())
    }

    fn cert_chain(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub seed: u64,
    pub key_id: String,
    pub c2pa_cert: String,
    pub ca_cert: String,
}

impl Saved {
    pub fn new(seed: u64, e: &Enrolled) -> Self {
        Self {
            seed,
            key_id: hex::encode(&e.key_id),
            c2pa_cert: hex::encode(&e.c2pa_cert),
            ca_cert: hex::encode(&e.ca_cert),
        }
    }

    pub fn enrolled(&self) -> Result<Enrolled, hex::FromHexError> {
        Ok(Enrolled {
            key_id: hex::decode(&self.key_id)?,
            c2pa_cert: hex::decode(&self.c2pa_cert)?,
            ca_cert: hex::decode(&self.ca_cert)?,
        })
    }
}

pub fn enrollment(challenge: &[u8], seed: u64, now_s: i64) -> Enrollment {
    let a = attest::Attestation::new(challenge, now_s);
    Enrollment {
        key_id: None,
        chain: a
            .chain(&attest::device_key(seed))
            .into_iter()
            .map(ByteBuf::from)
            .collect(),
        late: None,
    }
}

pub fn device(key: SigningKey, e: &Enrolled) -> Result<(TestDevice, TrustAnchor), String> {
    let anchor = TrustAnchor::from_der(&e.ca_cert).map_err(|err| format!("ca cert: {err:?}"))?;
    let leaf = verify_chain(&[&e.c2pa_cert, &e.ca_cert], &anchor, None)
        .map_err(|err| format!("c2pa chain: {err:?}"))?;
    if leaf.spki_der != attest::spki(&key) {
        return Err("issued certificate is for another key".into());
    }
    let serial = leaf
        .serial
        .iter()
        .fold(0u64, |a, b| (a << 8) | u64::from(*b));
    Ok((
        TestDevice {
            key,
            spki: leaf.spki_der,
            serial,
            chain: vec![e.c2pa_cert.clone(), e.ca_cert.clone()],
        },
        anchor,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::Verifier as _;
    use witness_verdict::testkit::TestCa;

    fn issued(seed: u8) -> (SigningKey, Enrolled) {
        let d = TestCa::new(2).device(seed, 0x51);
        let e = Enrolled {
            key_id: d.key_id().as_bytes().to_vec(),
            c2pa_cert: d.chain[0].clone(),
            ca_cert: d.chain[1].clone(),
        };
        (d.key, e)
    }

    #[test]
    fn signer_emits_verifiable_der() {
        let s = SmokeSigner(attest::device_key(4));
        let sig = s.sign(b"witness/erase/v1".to_vec()).unwrap();
        let parsed = p256::ecdsa::Signature::from_der(&sig).unwrap();
        s.0.verifying_key()
            .verify(b"witness/erase/v1", &parsed)
            .unwrap();
        assert_eq!(sig, s.sign(b"witness/erase/v1".to_vec()).unwrap());
        assert!(s.cert_chain().is_empty());
    }

    #[test]
    fn enrollment_carries_the_challenge_chain() {
        let e = enrollment(b"challenge", 3, 1_800_000_000);
        assert_eq!(e.chain.len(), 3);
        assert_eq!(e.key_id, None);
        assert!(e.chain[0].windows(9).any(|w| w == b"challenge"));
        assert_eq!(e, enrollment(b"challenge", 3, 1_800_000_000));
        assert_ne!(
            e.chain[0],
            enrollment(b"challenge", 4, 1_800_000_000).chain[0]
        );
    }

    #[test]
    fn saved_round_trips() {
        let e = Enrolled {
            key_id: vec![1; 16],
            c2pa_cert: vec![2, 3],
            ca_cert: vec![4],
        };
        let s = Saved::new(9, &e);
        let json = serde_json::to_string(&s).unwrap();
        let back: Saved = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.enrolled().unwrap(), e);
        let bad = Saved {
            key_id: "zz".into(),
            ..s
        };
        assert!(bad.enrolled().is_err());
    }

    #[test]
    fn device_accepts_matching_issued_chain() {
        let (key, e) = issued(7);
        let (d, _) = device(key.clone(), &e).unwrap();
        assert_eq!(d.spki, attest::spki(&key));
        assert_eq!(d.serial, 0x51);
        assert_eq!(d.key_id().as_bytes().as_slice(), e.key_id.as_slice());
        assert_eq!(d.chain, vec![e.c2pa_cert.clone(), e.ca_cert.clone()]);
    }

    #[test]
    fn device_rejects_wrong_key_and_garbage() {
        let (key, e) = issued(7);
        assert!(device(attest::device_key(8), &e).is_err());
        let mut junk = e.clone();
        junk.ca_cert = vec![0x30, 0x00];
        assert!(device(key.clone(), &junk).is_err());
        let mut junk = e;
        junk.c2pa_cert = vec![1, 2, 3];
        assert!(device(key, &junk).is_err());
    }
}
