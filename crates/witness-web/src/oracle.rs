use crate::error::{Error, Result};
use crate::wire::{Dec, Fetched, FingerprintsWire, Hex};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use witness_core::fingerprint::PdqHash;
use witness_core::format::{CaptureId, FingerprintBundle, KeyId, KeyInfo, RecordId};
use witness_core::verdict::{Oracle, PdqHit, PdqProbe, Record};

pub const STAGE_LOOKUP: u8 = 0;
pub const STAGE_RESOLVE: u8 = 1;
pub const STAGE_DETAIL: u8 = 2;
pub const STAGE_PIXELS: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactNeed {
    pub asset: Hex,
    pub pixel: Option<Hex>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Need {
    pub keys: BTreeSet<Hex>,
    pub records: BTreeSet<String>,
    pub record_ixs: BTreeSet<Dec>,
    pub exact: Option<ExactNeed>,
    pub watermark: BTreeSet<Dec>,
    pub pdq: Option<Vec<Hex>>,
    pub derivatives: BTreeSet<Dec>,
    pub fingerprints: BTreeSet<Dec>,
    pub references: BTreeSet<String>,
    #[serde(skip)]
    stage: Option<u8>,
}

impl Need {
    pub fn is_empty(&self) -> bool {
        self.stage.is_none()
    }

    pub fn stage(&self) -> Option<u8> {
        self.stage
    }

    fn ask(&mut self, stage: u8, add: impl FnOnce(&mut Self)) {
        if self.stage.is_some_and(|s| s < stage) {
            return;
        }
        add(self);
        self.stage = Some(self.stage.map_or(stage, |s| s.min(stage)));
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    keys: BTreeMap<KeyId, Option<KeyInfo>>,
    records: BTreeMap<RecordId, Option<Record>>,
    by_ix: BTreeMap<u64, RecordId>,
    absent_ixs: BTreeSet<u64>,
    exact: Option<Vec<u64>>,
    watermark: BTreeMap<u64, Vec<u64>>,
    pdq: Option<(Vec<(u64, u32)>, bool)>,
    derivatives: BTreeMap<u64, Vec<u64>>,
    fingerprints: BTreeMap<u64, Option<FingerprintsWire>>,
    references: BTreeMap<RecordId, Option<Vec<u8>>>,
    index_pending: bool,
}

fn ixs(list: Vec<Dec>) -> Vec<u64> {
    list.into_iter().map(|d| d.0).collect()
}

fn record_id(text: &str) -> Result<RecordId> {
    RecordId::parse(text).map_err(|_| Error::Malformed("record id"))
}

impl Answers {
    pub fn parse(json: &str) -> Result<Self> {
        Self::from_fetched(serde_json::from_str(json)?)
    }

    pub fn from_fetched(f: Fetched) -> Result<Self> {
        let mut keys = BTreeMap::new();
        for (id, key) in f.keys {
            let key_id =
                KeyId::try_from(id.0.as_slice()).map_err(|_| Error::Malformed("key id"))?;
            let info = key.map(|k| k.to_info()).transpose()?;
            if info.as_ref().is_some_and(|k| k.key_id != key_id) {
                return Err(Error::Malformed("key id"));
            }
            keys.insert(key_id, info);
        }
        let mut records = BTreeMap::new();
        let mut by_ix = BTreeMap::new();
        for (id, wire) in f.records {
            let id = record_id(&id)?;
            let record = wire.map(|w| w.to_record()).transpose()?;
            if let Some(r) = &record {
                if r.record_id != id {
                    return Err(Error::Malformed("record id"));
                }
                if by_ix.insert(r.record_ix, id).is_some() {
                    return Err(Error::Malformed("duplicate record index"));
                }
            }
            records.insert(id, record);
        }
        let references = f
            .references
            .into_iter()
            .map(|(id, img)| Ok((record_id(&id)?, img.map(|b| b.0))))
            .collect::<Result<_>>()?;
        Ok(Self {
            keys,
            records,
            by_ix,
            absent_ixs: f.absent_ixs.into_iter().map(|d| d.0).collect(),
            exact: f.exact.map(ixs),
            watermark: f
                .watermark
                .into_iter()
                .map(|(k, v)| (k.0, ixs(v)))
                .collect(),
            pdq: f.pdq.map(|p| {
                (
                    p.hits.iter().map(|h| (h.ix.0, h.distance)).collect(),
                    p.truncated,
                )
            }),
            derivatives: f
                .derivatives
                .into_iter()
                .map(|(k, v)| (k.0, ixs(v)))
                .collect(),
            fingerprints: f.fingerprints.into_iter().map(|(k, v)| (k.0, v)).collect(),
            references,
            index_pending: f.index_pending,
        })
    }
}

pub struct ReplayOracle<'a> {
    answers: &'a Answers,
    need: RefCell<Need>,
}

impl<'a> ReplayOracle<'a> {
    pub fn new(answers: &'a Answers) -> Self {
        Self {
            answers,
            need: RefCell::new(Need::default()),
        }
    }

    pub fn into_need(self) -> Need {
        self.need.into_inner()
    }

    fn ask(&self, stage: u8, add: impl FnOnce(&mut Need)) {
        self.need.borrow_mut().ask(stage, add);
    }

    fn by_ix(&self, ix: u64) -> Option<Record> {
        if let Some(id) = self.answers.by_ix.get(&ix) {
            return self.answers.records.get(id).cloned().flatten();
        }
        if !self.answers.absent_ixs.contains(&ix) {
            self.ask(STAGE_RESOLVE, |n| {
                n.record_ixs.insert(Dec(ix));
            });
        }
        None
    }

    fn resolve(&self, list: &[u64]) -> Vec<Record> {
        list.iter().filter_map(|&ix| self.by_ix(ix)).collect()
    }
}

impl Oracle for ReplayOracle<'_> {
    fn get_key(&self, key_id: &KeyId) -> Option<KeyInfo> {
        if let Some(k) = self.answers.keys.get(key_id) {
            return k.clone();
        }
        self.ask(STAGE_LOOKUP, |n| {
            n.keys.insert(Hex(key_id.as_bytes().to_vec()));
        });
        None
    }

    fn get_record(&self, record_id: &RecordId) -> Option<Record> {
        if let Some(r) = self.answers.records.get(record_id) {
            return r.clone();
        }
        self.ask(STAGE_LOOKUP, |n| {
            n.records.insert(record_id.to_string());
        });
        None
    }

    fn exact(&self, asset_hash: &[u8; 32], pixel_hash: Option<&[u8; 32]>) -> Vec<Record> {
        if let Some(list) = &self.answers.exact {
            return self.resolve(list);
        }
        self.ask(STAGE_LOOKUP, |n| {
            n.exact = Some(ExactNeed {
                asset: Hex(asset_hash.to_vec()),
                pixel: pixel_hash.map(|p| Hex(p.to_vec())),
            });
        });
        Vec::new()
    }

    fn by_watermark(&self, capture_id: CaptureId) -> Vec<Record> {
        if let Some(list) = self.answers.watermark.get(&capture_id.to_u64()) {
            return self.resolve(list);
        }
        self.ask(STAGE_LOOKUP, |n| {
            n.watermark.insert(Dec(capture_id.to_u64()));
        });
        Vec::new()
    }

    fn pdq_probe(&self, hashes: &[PdqHash]) -> PdqProbe {
        if let Some((hits, truncated)) = &self.answers.pdq {
            return PdqProbe {
                hits: hits
                    .iter()
                    .filter_map(|&(ix, distance)| {
                        self.by_ix(ix).map(|record| PdqHit { record, distance })
                    })
                    .collect(),
                truncated: *truncated,
            };
        }
        self.ask(STAGE_LOOKUP, |n| {
            n.pdq = Some(hashes.iter().map(|h| Hex(h.0.to_vec())).collect());
        });
        PdqProbe::default()
    }

    fn derivatives(&self, root: &Record) -> Vec<Record> {
        if let Some(list) = self.answers.derivatives.get(&root.record_ix) {
            return self.resolve(list);
        }
        self.ask(STAGE_RESOLVE, |n| {
            n.derivatives.insert(Dec(root.record_ix));
        });
        Vec::new()
    }

    fn reference_image(&self, record: &Record) -> Option<Vec<u8>> {
        if let Some(img) = self.answers.references.get(&record.record_id) {
            return img.clone();
        }
        self.ask(STAGE_PIXELS, |n| {
            n.references.insert(record.record_id.to_string());
        });
        None
    }

    fn fingerprints(&self, record: &Record) -> Option<FingerprintBundle> {
        if let Some(f) = self.answers.fingerprints.get(&record.record_ix) {
            return f.as_ref().and_then(|w| w.bundle_for(&record.claim));
        }
        self.ask(STAGE_DETAIL, |n| {
            n.fingerprints.insert(Dec(record.record_ix));
        });
        None
    }

    fn index_pending(&self) -> bool {
        self.answers.index_pending
    }
}

#[cfg(test)]
mod tests;
