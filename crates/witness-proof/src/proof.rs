use crate::certificate::{self, Certified, MAX_PRINCIPAL_LEN};
use crate::error::{Error, Result};
use alloc::vec::Vec;
use witness_format::crypto::{hash_ds, sha256};
use witness_format::{KeyInfo, RecordId};
use witness_merkle::hash::{mmr_leaf, smt_leaf};
use witness_merkle::{Hash, InclusionProof, SmtProof, certified_data, key_of};

pub const SHARDS_KEY: u128 = u128::MAX;
pub const SHARDS_DOMAIN: &[u8] = b"witness/shards/v1";
pub const ROOTS_LEN: usize = 32 + 8;
pub const LEAF_DATA_LEN: usize = 32 * 3 + 8;

pub fn leaf_data(
    claim: &[u8],
    c2pa_claim: &[u8],
    cose: &[u8],
    anchored_ns: u64,
) -> [u8; LEAF_DATA_LEN] {
    let mut out = [0u8; LEAF_DATA_LEN];
    out[..32].copy_from_slice(&sha256(claim));
    out[32..64].copy_from_slice(&sha256(c2pa_claim));
    out[64..96].copy_from_slice(&sha256(cose));
    out[96..].copy_from_slice(&anchored_ns.to_be_bytes());
    out
}

#[derive(Debug, Clone, Copy)]
pub struct RecordParts<'a, H> {
    pub record_ix: u64,
    pub record_id: &'a str,
    pub claim: &'a [u8],
    pub c2pa_claim: &'a [u8],
    pub cose: &'a [u8],
    pub anchored_ns: u64,
    pub leaf_count: u64,
    pub mmr_proof: &'a [H],
    pub certificate: &'a [u8],
}

fn hash_of(bytes: &[u8]) -> Result<Hash> {
    bytes
        .try_into()
        .map_err(|_| Error::Malformed("record proof"))
}

impl<H: AsRef<[u8]>> RecordParts<'_, H> {
    pub fn record_id(&self) -> Result<RecordId> {
        let derived = RecordId::from_claim_cbor(self.claim);
        match RecordId::parse(self.record_id) {
            Ok(id) if id == derived => Ok(id),
            _ => Err(Error::Malformed("record id")),
        }
    }

    pub fn root(&self) -> Result<Hash> {
        let bad = Error::Proof("record");
        let (smt_root, path) = self.mmr_proof.split_last().ok_or(bad)?;
        let smt_root = hash_of(smt_root.as_ref())?;
        let path = path
            .iter()
            .map(|h| hash_of(h.as_ref()))
            .collect::<Result<Vec<Hash>>>()?;
        let proof = InclusionProof {
            index: self.record_ix,
            leaf_count: self.leaf_count,
            path,
        };
        let leaf = mmr_leaf(&leaf_data(
            self.claim,
            self.c2pa_claim,
            self.cose,
            self.anchored_ns,
        ));
        let mmr_root = proof.compute_root(&leaf).ok_or(bad)?;
        Ok(certified_data(&mmr_root, self.leaf_count, &smt_root))
    }
}

pub fn smt_bundle_root(bundle: &[u8], key: u128, value: &Hash) -> Result<Hash> {
    let bad = Error::Proof("smt bundle");
    let split = bundle.len().checked_sub(ROOTS_LEN).ok_or(bad)?;
    let (proof, tail) = bundle.split_at(split);
    let (mmr_root, leaf_count) = tail.split_first_chunk::<32>().ok_or(bad)?;
    let leaf_count = u64::from_be_bytes(leaf_count.try_into().map_err(|_| bad)?);
    let proof = SmtProof::from_bytes(proof).map_err(|_| bad)?;
    let smt_root = proof.compute_root(key, &smt_leaf(key, value)).ok_or(bad)?;
    Ok(certified_data(mmr_root, leaf_count, &smt_root))
}

pub fn key_root(info: &KeyInfo, smt_bundle: &[u8]) -> Result<Hash> {
    smt_bundle_root(smt_bundle, key_of(&info.key_id), &info.leaf_hash())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardEntry<'a> {
    pub shard: u8,
    pub canister: &'a [u8],
    pub open: bool,
}

fn push_shards<'a, I>(buf: &mut Vec<u8>, list: I) -> Result<()>
where
    I: IntoIterator<Item = ShardEntry<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let list = list.into_iter();
    let len = u16::try_from(list.len()).map_err(|_| Error::Malformed("shard map"))?;
    buf.extend_from_slice(&len.to_be_bytes());
    for e in list {
        let len = u8::try_from(e.canister.len())
            .ok()
            .filter(|&l| usize::from(l) <= MAX_PRINCIPAL_LEN)
            .ok_or(Error::Malformed("canister id"))?;
        buf.extend_from_slice(&[e.shard, u8::from(e.open), len]);
        buf.extend_from_slice(e.canister);
    }
    Ok(())
}

pub fn shard_map_hash<'a, I, M>(index: I, media: M) -> Result<Hash>
where
    I: IntoIterator<Item = ShardEntry<'a>>,
    I::IntoIter: ExactSizeIterator,
    M: IntoIterator<Item = ShardEntry<'a>>,
    M::IntoIter: ExactSizeIterator,
{
    let mut buf = Vec::with_capacity(4 + 32 * 4);
    push_shards(&mut buf, index)?;
    push_shards(&mut buf, media)?;
    Ok(hash_ds(SHARDS_DOMAIN, &[&buf]))
}

pub fn split_shards_bundle(bundle: &[u8]) -> Result<(&[u8], &[u8])> {
    let bad = Error::Proof("shards bundle");
    let (len, rest) = bundle.split_first_chunk::<4>().ok_or(bad)?;
    let len = usize::try_from(u32::from_be_bytes(*len)).map_err(|_| bad)?;
    rest.split_at_checked(len).ok_or(bad)
}

pub fn shards_root<'b>(bundle: &'b [u8], map_hash: &Hash) -> Result<(&'b [u8], Hash)> {
    let (cert, smt) = split_shards_bundle(bundle)?;
    Ok((cert, smt_bundle_root(smt, SHARDS_KEY, map_hash)?))
}

#[derive(Debug, Clone, Copy)]
pub struct Trust<'a> {
    pub root_key: &'a [u8],
    pub canister: &'a [u8],
    pub now_ns: u64,
    pub max_age_ns: u64,
}

impl Trust<'_> {
    pub fn certify(&self, certificate: &[u8], expected: &Hash) -> Result<Certified> {
        let c = certificate::verify(
            certificate,
            self.root_key,
            self.canister,
            self.now_ns,
            self.max_age_ns,
        )?;
        if c.data.as_slice() == expected.as_slice() {
            Ok(c)
        } else {
            Err(Error::Proof("certified data mismatch"))
        }
    }

    pub fn record<H: AsRef<[u8]>>(&self, r: &RecordParts<'_, H>) -> Result<Certified> {
        r.record_id()?;
        self.certify(r.certificate, &r.root()?)
    }

    pub fn key(&self, info: &KeyInfo, smt_bundle: &[u8], certificate: &[u8]) -> Result<Certified> {
        self.certify(certificate, &key_root(info, smt_bundle)?)
    }

    pub fn shards(&self, bundle: &[u8], map_hash: &Hash) -> Result<Certified> {
        let (cert, root) = shards_root(bundle, map_hash)?;
        self.certify(cert, &root)
    }
}

#[cfg(test)]
mod tests;
