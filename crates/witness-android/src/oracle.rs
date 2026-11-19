use crate::client::{IcpClient, MAX_PDQ_HITS};
use crate::error::WitnessError;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use tokio::runtime::Handle;
use witness_canister_common::{CertifiedRecord, ShardInfo, Shards};
use witness_core::fingerprint::PdqHash;
use witness_core::format::{
    CaptureId, FingerprintBundle, KeyId, KeyInfo, PdqShape, RecordId, WitnessClaim,
};
use witness_core::verdict::{
    MAX_DERIVATIVES, MAX_EXACT, MAX_WATERMARK, Oracle, PdqHit, PdqProbe, Record,
};
use witness_proof::Trust;

pub(crate) struct NetOracle<'a> {
    rt: &'a Handle,
    client: &'a IcpClient,
    shards: &'a Shards,
    trust: Trust<'a>,
    failure: Mutex<Option<WitnessError>>,
    rejected: AtomicBool,
}

pub(crate) fn to_record(r: CertifiedRecord) -> Option<Record> {
    let record_id = RecordId::parse(&r.record_id).ok()?;
    if RecordId::from_claim_cbor(&r.claim) != record_id {
        return None;
    }
    let claim = WitnessClaim::from_cbor(&r.claim).ok()?;
    Some(Record {
        record_ix: r.record_ix,
        record_id,
        flags: witness_core::verdict::clock_flags(&claim, r.anchored_ns),
        claim,
        anchored_ns: r.anchored_ns,
    })
}

pub(crate) fn to_bundle(
    f: witness_canister_common::Fingerprints,
    portrait: bool,
) -> Option<FingerprintBundle> {
    let grid_bytes: &[u8; 384] = f.grid.as_slice().try_into().ok()?;
    let grid =
        core::array::from_fn(|i| u16::from_be_bytes([grid_bytes[2 * i], grid_bytes[2 * i + 1]]));
    let pdq = f
        .shapes
        .into_iter()
        .map(|(tag, h)| {
            Some(PdqShape {
                tag,
                hash: h.as_slice().try_into().ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    FingerprintBundle::new(pdq, 0, grid, portrait).ok()
}

impl<'a> NetOracle<'a> {
    pub(crate) fn new(
        rt: &'a Handle,
        client: &'a IcpClient,
        shards: &'a Shards,
        trust: Trust<'a>,
    ) -> Self {
        Self {
            rt,
            client,
            shards,
            trust,
            failure: Mutex::new(None),
            rejected: AtomicBool::new(false),
        }
    }

    pub(crate) fn rejected(&self) -> bool {
        self.rejected.load(Ordering::Relaxed)
    }

    fn reject<T>(&self) -> Option<T> {
        self.rejected.store(true, Ordering::Relaxed);
        None
    }

    fn verified(&self, r: CertifiedRecord, wanted: impl FnOnce(&Record) -> bool) -> Option<Record> {
        if self.trust.certified_record(&r).is_err() {
            return self.reject();
        }
        to_record(r).filter(wanted).or_else(|| self.reject())
    }

    pub(crate) fn into_failure(self) -> Option<WitnessError> {
        self.failure
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn index(&self) -> &[ShardInfo] {
        &self.shards.index
    }

    fn run<T>(&self, f: impl Future<Output = Result<T, WitnessError>>) -> Option<T> {
        match self.rt.block_on(f) {
            Ok(v) => Some(v),
            Err(e) => {
                self.failure
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .get_or_insert(e);
                None
            }
        }
    }

    fn records(&self, ixs: &[u64], cap: usize) -> Vec<Record> {
        let ixs = &ixs[..ixs.len().min(cap)];
        self.run(self.client.records(ixs))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| self.verified(r, |r| ixs.contains(&r.record_ix)))
            .collect()
    }
}

impl Oracle for NetOracle<'_> {
    fn get_key(&self, key_id: &KeyId) -> Option<KeyInfo> {
        let k = self.run(self.client.get_key(key_id.as_bytes()))??;
        match self.trust.certified_key(&k) {
            Ok((info, _)) if info.key_id == *key_id => Some(info),
            _ => self.reject(),
        }
    }

    fn get_record(&self, record_id: &RecordId) -> Option<Record> {
        let r = self.run(self.client.get(&record_id.to_string()))??;
        self.verified(r, |r| r.record_id == *record_id)
    }

    fn exact(&self, asset_hash: &[u8; 32], pixel_hash: Option<&[u8; 32]>) -> Vec<Record> {
        let hashes: Vec<Vec<u8>> = core::iter::once(asset_hash)
            .chain(pixel_hash)
            .map(|h| h.to_vec())
            .collect();
        self.run(self.client.probe(self.index(), &[], &[], &hashes))
            .map(|p| self.records(&p.exact, MAX_EXACT))
            .unwrap_or_default()
    }

    fn by_watermark(&self, capture_id: CaptureId) -> Vec<Record> {
        self.run(
            self.client
                .probe(self.index(), &[], &[capture_id.to_u64()], &[]),
        )
        .map(|p| self.records(&p.watermark, MAX_WATERMARK))
        .unwrap_or_default()
    }

    fn pdq_probe(&self, hashes: &[PdqHash]) -> PdqProbe {
        let hashes: Vec<Vec<u8>> = hashes.iter().map(|h| h.bytes().to_vec()).collect();
        let Some(probe) = self.run(self.client.probe(self.index(), &hashes, &[], &[])) else {
            return PdqProbe::default();
        };
        let ixs: Vec<u64> = probe.pdq.iter().map(|h| h.record_ix).collect();
        let records = self.records(&ixs, MAX_PDQ_HITS);
        let hits = probe
            .pdq
            .iter()
            .filter_map(|h| {
                records
                    .iter()
                    .find(|r| r.record_ix == h.record_ix)
                    .map(|r| PdqHit {
                        record: r.clone(),
                        distance: h.distance,
                    })
            })
            .collect();
        PdqProbe {
            hits,
            truncated: probe.truncated_candidates,
        }
    }

    fn derivatives(&self, root: &Record) -> Vec<Record> {
        self.run(
            self.client
                .derivatives(self.index(), root.record_ix, MAX_DERIVATIVES),
        )
        .map(|ixs| self.records(&ixs, MAX_DERIVATIVES))
        .unwrap_or_default()
    }

    fn reference_image(&self, record: &Record) -> Option<Vec<u8>> {
        if record.claim.kind.is_original() {
            return None;
        }
        let path = format!("/p/{}", record.record_id);
        self.run(self.client.http_get_any(&self.shards.media, &path))?
    }

    fn fingerprints(&self, record: &Record) -> Option<FingerprintBundle> {
        let dims = record.claim.dims;
        let f = self.run(self.client.fingerprints(self.index(), record.record_ix))??;
        to_bundle(f, dims.height > dims.width)
    }
}

#[cfg(test)]
pub(crate) mod tests;
