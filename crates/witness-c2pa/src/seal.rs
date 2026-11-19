use crate::Error;
use crate::app11::{self, MAX_CHUNK};
use crate::assertions::{
    self, ACTIONS, Action, DATA_HASH, Exclusion, WITNESS_CLAIM, box_hash, cbor_assertion_box,
};
use crate::claim::{self, Generator, HashedUri};
use crate::cose;
use crate::manifest::write_store;
use crate::x509::{self, TrustAnchor};
use alloc::vec::Vec;
use witness_format::crypto::{ES256_RAW_LEN, es256_sig_to_raw};
use witness_format::{KeyId, WitnessClaim};

pub trait Signer {
    fn sign(&self, data: &[u8]) -> Vec<u8>;
    fn cert_chain(&self) -> Vec<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestSpec<'a> {
    pub id: [u8; 16],
    pub generator: Generator<'a>,
    pub actions: &'a [Action<'a>],
    pub witness_claim: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    pub jpeg: Vec<u8>,
    pub data_hash: [u8; 32],
    pub c2pa_claim: Vec<u8>,
    pub cose_sign1: Vec<u8>,
    pub manifest_label: alloc::string::String,
}

pub fn seal_jpeg(
    jpeg: &[u8],
    spec: &ManifestSpec<'_>,
    signer: &dyn Signer,
    anchor: &TrustAnchor,
) -> Result<Sealed, Error> {
    seal_jpeg_chunked(jpeg, spec, signer, anchor, MAX_CHUNK)
}

struct Fixed {
    label: alloc::string::String,
    instance_id: alloc::string::String,
    signature_uri: alloc::string::String,
    actions_box: Vec<u8>,
    witness_box: Vec<u8>,
    urls: [alloc::string::String; 3],
    protected: Vec<u8>,
}

impl Fixed {
    fn claim(&self, spec: &ManifestSpec<'_>, data_hash_box: &[u8]) -> Vec<u8> {
        let hashes = [
            box_hash(&self.actions_box),
            box_hash(data_hash_box),
            box_hash(&self.witness_box),
        ];
        let uris: [HashedUri<'_>; 3] = core::array::from_fn(|i| HashedUri {
            url: &self.urls[i],
            alg: None,
            hash: &hashes[i],
        });
        let mut out = Vec::with_capacity(512);
        claim::write_claim(
            &mut out,
            &self.instance_id,
            &spec.generator,
            &self.signature_uri,
            &uris,
        );
        out
    }

    fn store(&self, data_hash_box: &[u8], claim: &[u8], cose: &[u8]) -> Result<Vec<u8>, Error> {
        let boxes = [
            self.actions_box.as_slice(),
            data_hash_box,
            self.witness_box.as_slice(),
        ];
        let mut out = Vec::with_capacity(
            boxes.iter().map(|b| b.len()).sum::<usize>() + claim.len() + cose.len() + 256,
        );
        write_store(&mut out, &self.label, &boxes, claim, cose)?;
        Ok(out)
    }
}

fn check_claim(claim: &WitnessClaim, data_hash: &[u8; 32], leaf: &x509::Leaf) -> Result<(), Error> {
    if &claim.asset_hash != data_hash {
        return Err(Error::AssetHashMismatch);
    }
    if claim.key.key_id != KeyId::from_spki(&leaf.spki_der) {
        return Err(Error::KeyMismatch);
    }
    if claim.cert_serial.as_bytes() != leaf.serial.as_slice() {
        return Err(Error::SerialMismatch);
    }
    Ok(())
}

pub(crate) fn seal_jpeg_chunked(
    jpeg: &[u8],
    spec: &ManifestSpec<'_>,
    signer: &dyn Signer,
    anchor: &TrustAnchor,
    chunk: usize,
) -> Result<Sealed, Error> {
    let layout = app11::locate(jpeg)?;
    let data_hash = app11::hash_excluding(jpeg, &layout.excluded);
    let chain = signer.cert_chain();
    let chain_refs: Vec<&[u8]> = chain.iter().map(Vec::as_slice).collect();
    let leaf = x509::verify_chain(&chain_refs, anchor, None)?;
    let witness = WitnessClaim::from_cbor(spec.witness_claim).map_err(Error::WitnessClaim)?;
    check_claim(&witness, &data_hash, &leaf)?;

    let label = claim::manifest_label(&spec.id);
    let fixed = Fixed {
        instance_id: claim::instance_id(&spec.id),
        signature_uri: claim::signature_uri(&label),
        actions_box: cbor_assertion_box(ACTIONS, &assertions::actions_cbor(spec.actions)?)?,
        witness_box: cbor_assertion_box(WITNESS_CLAIM, spec.witness_claim)?,
        urls: [ACTIONS, DATA_HASH, WITNESS_CLAIM].map(|a| claim::assertion_uri(&label, a)),
        protected: cose::protected_header(&chain),
        label,
    };
    let start = layout.insert_at as u64;
    let placeholder_cose = cose::encode_sign1(&fixed.protected, &[0; ES256_RAW_LEN]);
    let mut length = 0u64;
    let (hash_box, c2pa_claim, store_len) = loop {
        let hash_box = cbor_assertion_box(
            DATA_HASH,
            &assertions::data_hash_cbor(Exclusion { start, length }, &data_hash),
        )?;
        let claim = fixed.claim(spec, &hash_box);
        let store_len = fixed.store(&hash_box, &claim, &placeholder_cose)?.len();
        let needed = app11::packetized_len(store_len, chunk) as u64;
        if needed == length {
            break (hash_box, claim, store_len);
        }
        if needed < length {
            return Err(Error::TooLarge);
        }
        length = needed;
    };

    let raw = es256_sig_to_raw(&signer.sign(&cose::sig_structure(&fixed.protected, &c2pa_claim)))
        .map_err(|_| Error::Signer)?;
    let cose_sign1 = cose::encode_sign1(&fixed.protected, &raw);
    cose::Sign1::parse(&cose_sign1)?
        .verify(&c2pa_claim, &leaf.key)
        .map_err(|_| Error::Signer)?;
    let store = fixed.store(&hash_box, &c2pa_claim, &cose_sign1)?;
    if store.len() != store_len {
        return Err(Error::Signer);
    }

    let excluded_len: usize = layout.excluded.iter().map(|r| r.len()).sum();
    let mut out = Vec::with_capacity(jpeg.len() - excluded_len + length as usize);
    out.extend_from_slice(&jpeg[..layout.insert_at]);
    app11::write_packets(&mut out, &store, layout.free_en(), chunk)?;
    let mut at = layout.insert_at;
    for r in &layout.excluded {
        out.extend_from_slice(&jpeg[at..r.start]);
        at = r.end;
    }
    out.extend_from_slice(&jpeg[at..]);
    Ok(Sealed {
        jpeg: out,
        data_hash,
        c2pa_claim,
        cose_sign1,
        manifest_label: fixed.label,
    })
}

#[cfg(test)]
mod tests;
