use crate::certificate::Certified;
use crate::error::{Error, Result};
use crate::proof::{RecordParts, Trust};
use serde_bytes::ByteBuf;
use witness_canister_common::{CertifiedKey, CertifiedRecord};
use witness_format::KeyInfo;

pub fn record_parts(r: &CertifiedRecord) -> RecordParts<'_, ByteBuf> {
    RecordParts {
        record_ix: r.record_ix,
        record_id: &r.record_id,
        claim: &r.claim,
        c2pa_claim: &r.c2pa_claim,
        cose: &r.cose,
        anchored_ns: r.anchored_ns,
        leaf_count: r.leaf_count,
        mmr_proof: &r.mmr_proof,
        certificate: &r.certificate,
    }
}

pub fn key_info(k: &CertifiedKey) -> Result<KeyInfo> {
    KeyInfo::try_from(k.key.clone()).map_err(|_| Error::Malformed("key"))
}

impl Trust<'_> {
    pub fn certified_record(&self, r: &CertifiedRecord) -> Result<Certified> {
        self.record(&record_parts(r))
    }

    pub fn certified_key(&self, k: &CertifiedKey) -> Result<(KeyInfo, Certified)> {
        let info = key_info(k)?;
        let c = self.key(&info, &k.smt_proof, &k.certificate)?;
        Ok((info, c))
    }
}

#[cfg(test)]
mod tests;
