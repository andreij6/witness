use crate::config::{LEAF_SUBJECT, LEAF_VALIDITY_S, ROOT_SERIAL, ROOT_SUBJECT, ROOT_VALIDITY_S};
use witness_c2pa::TrustAnchor;
use witness_c2pa::cert::{CertParams, assemble, leaf_tbs, root_tbs};
use witness_format::CertSerial;
use witness_format::crypto::{ED25519_KEY_LEN, ED25519_SIG_LEN};

pub const NS_PER_S: u64 = 1_000_000_000;

fn params(
    subject: &str,
    serial: u64,
    now_ns: u64,
    validity_s: u64,
) -> Result<CertParams<'_>, String> {
    let not_before = now_ns / NS_PER_S;
    Ok(CertParams {
        subject,
        serial: CertSerial::from_u64(serial).ok_or("bad serial")?,
        not_before,
        not_after: not_before.saturating_add(validity_s),
    })
}

pub fn root_to_sign(now_ns: u64, public_key: &[u8]) -> Result<Vec<u8>, String> {
    let key: &[u8; ED25519_KEY_LEN] = public_key.try_into().map_err(|_| "bad ca key")?;
    let p = params(ROOT_SUBJECT, ROOT_SERIAL, now_ns, ROOT_VALIDITY_S)?;
    root_tbs(&p, key).map_err(|e| e.to_string())
}

pub fn leaf_to_sign(
    serial: u64,
    now_ns: u64,
    spki: &[u8],
    anchor: &TrustAnchor,
) -> Result<Vec<u8>, String> {
    let p = params(LEAF_SUBJECT, serial, now_ns, LEAF_VALIDITY_S)?;
    leaf_tbs(&p, spki, anchor).map_err(|e| e.to_string())
}

pub fn finish(tbs: &[u8], signature: &[u8]) -> Result<Vec<u8>, String> {
    let sig: &[u8; ED25519_SIG_LEN] = signature.try_into().map_err(|_| "bad signature length")?;
    assemble(tbs, sig).map_err(|e| e.to_string())
}

pub fn finish_root(tbs: &[u8], signature: &[u8], public_key: &[u8]) -> Result<Vec<u8>, String> {
    let der = finish(tbs, signature)?;
    let anchor = TrustAnchor::from_der(&der).map_err(|e| format!("{e:?}"))?;
    if anchor.public_key().as_slice() != public_key {
        return Err("ca key mismatch".into());
    }
    Ok(der)
}

pub fn anchor(root: &[u8]) -> Result<TrustAnchor, String> {
    TrustAnchor::from_der(root).map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer as _;
    use witness_c2pa::x509::verify_chain;

    pub const NOW: u64 = 1_763_510_400 * NS_PER_S;

    fn ca(seed: u8) -> (ed25519_dalek::SigningKey, Vec<u8>) {
        let k = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let pk = k.verifying_key().to_bytes();
        let tbs = root_to_sign(NOW, &pk).unwrap();
        let der = finish_root(&tbs, &k.sign(&tbs).to_bytes(), &pk).unwrap();
        (k, der)
    }

    fn spki(seed: u8) -> Vec<u8> {
        let k = p256::ecdsa::SigningKey::from_slice(&[seed; 32]).unwrap();
        let mut out = vec![
            0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06,
            0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
        ];
        out.extend_from_slice(k.verifying_key().to_encoded_point(false).as_bytes());
        out
    }

    #[test]
    fn root_and_leaf_verify_as_c2pa_chain() {
        let (k, root) = ca(1);
        let a = anchor(&root).unwrap();
        let device = spki(2);
        let tbs = leaf_to_sign(7, NOW, &device, &a).unwrap();
        let leaf = finish(&tbs, &k.sign(&tbs).to_bytes()).unwrap();
        let checked = verify_chain(&[&leaf, &root], &a, Some(NOW / NS_PER_S + 3600)).unwrap();
        assert_eq!(checked.spki_der, device);
        assert_eq!(checked.serial, vec![7]);
        assert!(verify_chain(&[&leaf], &a, Some(NOW / NS_PER_S + LEAF_VALIDITY_S + 1)).is_err());
        assert!(verify_chain(&[&leaf], &a, Some(NOW / NS_PER_S - 1)).is_err());
    }

    #[test]
    fn leaf_from_other_ca_is_rejected() {
        let (k1, root1) = ca(1);
        let (_, root2) = ca(3);
        let a1 = anchor(&root1).unwrap();
        let tbs = leaf_to_sign(7, NOW, &spki(2), &a1).unwrap();
        let leaf = finish(&tbs, &k1.sign(&tbs).to_bytes()).unwrap();
        assert!(verify_chain(&[&leaf], &anchor(&root2).unwrap(), None).is_err());
    }

    #[test]
    fn root_is_deterministic_for_same_inputs() {
        assert_eq!(ca(1).1, ca(1).1);
        assert_ne!(ca(1).1, ca(2).1);
    }

    #[test]
    fn root_rejects_wrong_signature_or_key() {
        let k = ed25519_dalek::SigningKey::from_bytes(&[1; 32]);
        let other = ed25519_dalek::SigningKey::from_bytes(&[2; 32]);
        let pk = k.verifying_key().to_bytes();
        let tbs = root_to_sign(NOW, &pk).unwrap();
        assert!(finish_root(&tbs, &other.sign(&tbs).to_bytes(), &pk).is_err());
        let sig = k.sign(&tbs).to_bytes();
        assert!(finish_root(&tbs, &sig, &other.verifying_key().to_bytes()).is_err());
        assert!(finish_root(&tbs, &sig[..63], &pk).is_err());
        assert!(root_to_sign(NOW, &pk[..31]).is_err());
    }

    #[test]
    fn leaf_rejects_bad_inputs() {
        let (_, root) = ca(1);
        let a = anchor(&root).unwrap();
        assert!(leaf_to_sign(0, NOW, &spki(2), &a).is_err());
        assert!(leaf_to_sign(7, NOW, b"not spki", &a).is_err());
        assert!(finish(b"not tbs", &[0; 64]).is_err());
        assert!(anchor(b"junk").is_err());
    }
}
