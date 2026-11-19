use crate::error::Result;
use crate::wire::{CertifiedKeyWire, Hex, RecordWire, ShardInfoWire, ShardsWire};
use serde_json::{Value, json};
use witness_core::merkle::Hash;
use witness_proof::proof as p;
pub use witness_proof::proof::{LEAF_DATA_LEN, ROOTS_LEN, SHARDS_DOMAIN, SHARDS_KEY, leaf_data};
pub use witness_proof::{Certified, Trust};

impl AsRef<[u8]> for Hex {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

fn parts(r: &RecordWire) -> p::RecordParts<'_, Hex> {
    p::RecordParts {
        record_ix: r.record_ix.0,
        record_id: &r.record_id,
        claim: &r.claim.0,
        c2pa_claim: &r.c2pa_claim.0,
        cose: &r.cose.0,
        anchored_ns: r.anchored_ns.0,
        leaf_count: r.leaf_count.0,
        mmr_proof: &r.mmr_proof,
        certificate: &r.certificate.0,
    }
}

fn entries(list: &[ShardInfoWire]) -> impl ExactSizeIterator<Item = p::ShardEntry<'_>> {
    list.iter().map(|i| p::ShardEntry {
        shard: i.shard,
        canister: &i.canister.0,
        open: i.open,
    })
}

pub fn record_root(r: &RecordWire) -> Result<Hash> {
    Ok(parts(r).root()?)
}

pub fn key_root(k: &CertifiedKeyWire) -> Result<Hash> {
    Ok(p::key_root(&k.key.to_info()?, &k.smt_proof.0)?)
}

pub fn shard_map_hash(s: &ShardsWire) -> Result<Hash> {
    Ok(p::shard_map_hash(entries(&s.index), entries(&s.media))?)
}

pub fn shards_root(s: &ShardsWire) -> Result<(&[u8], Hash)> {
    Ok(p::shards_root(&s.bundle.0, &shard_map_hash(s)?)?)
}

fn certified_json(c: &Certified, mut extra: Value) -> Value {
    extra["certifiedData"] = json!(hex::encode(&c.data));
    extra["certTimeNs"] = json!(c.time_ns.to_string());
    extra
}

pub fn verify_record(r: &RecordWire, trust: &Trust<'_>) -> Result<Value> {
    let parts = parts(r);
    let record_id = parts.record_id()?;
    let c = trust.record(&parts)?;
    Ok(certified_json(
        &c,
        json!({
            "recordId": record_id.to_string(),
            "recordIx": r.record_ix.0.to_string(),
            "anchoredNs": r.anchored_ns.0.to_string(),
            "leafCount": r.leaf_count.0.to_string(),
            "mediaShard": r.media_shard,
        }),
    ))
}

pub fn verify_key(k: &CertifiedKeyWire, trust: &Trust<'_>) -> Result<Value> {
    let c = trust.key(&k.key.to_info()?, &k.smt_proof.0, &k.certificate.0)?;
    Ok(certified_json(
        &c,
        json!({"keyId": hex::encode(&k.key.key_id.0)}),
    ))
}

pub fn verify_shards(s: &ShardsWire, trust: &Trust<'_>) -> Result<Value> {
    let c = trust.shards(&s.bundle.0, &shard_map_hash(s)?)?;
    Ok(certified_json(
        &c,
        json!({"index": s.index.len(), "media": s.media.len()}),
    ))
}

pub fn verify_record_json(json: &str, trust: &Trust<'_>) -> Result<String> {
    Ok(verify_record(&serde_json::from_str(json)?, trust)?.to_string())
}

pub fn verify_key_json(json: &str, trust: &Trust<'_>) -> Result<String> {
    Ok(verify_key(&serde_json::from_str(json)?, trust)?.to_string())
}

pub fn verify_shards_json(json: &str, trust: &Trust<'_>) -> Result<String> {
    Ok(verify_shards(&serde_json::from_str(json)?, trust)?.to_string())
}

#[cfg(test)]
mod tests;
