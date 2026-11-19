use crate::error::{Error, Result};
use base64::Engine;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::BTreeMap;
use witness_core::fingerprint::is_portrait;
use witness_core::format::fingerprint::{GRID_BYTES, GRID_CELLS, MAX_PDQ_QUALITY};
use witness_core::format::{
    FingerprintBundle, Freshness, KeyHistory, KeyId, KeyInfo, KeyStatus, PdqShape, Provisioning,
    RecordId, SecurityLevel, StatusChange, WitnessClaim,
};
use witness_core::verdict::Record;

const STANDARD_ANY_PADDING: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_encode_padding(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hex(pub Vec<u8>);

impl Hex {
    pub fn array<const N: usize>(&self, what: &'static str) -> Result<[u8; N]> {
        self.0
            .as_slice()
            .try_into()
            .map_err(|_| Error::Malformed(what))
    }
}

impl Serialize for Hex {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        let text = Cow::<str>::deserialize(d)?;
        hex::decode(text.as_ref())
            .map(Self)
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct B64(pub Vec<u8>);

impl Serialize for B64 {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD_ANY_PADDING.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for B64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        let text = Cow::<str>::deserialize(d)?;
        STANDARD_ANY_PADDING
            .decode(text.as_bytes())
            .map(Self)
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec(pub u64);

impl Serialize for Dec {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        s.collect_str(&self.0)
    }
}

struct DecVisitor;

impl Visitor<'_> for DecVisitor {
    type Value = Dec;

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("an unsigned 64-bit integer or its decimal string")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> core::result::Result<Dec, E> {
        Ok(Dec(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> core::result::Result<Dec, E> {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
            return Err(E::custom("not a decimal u64"));
        }
        v.parse().map(Dec).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Dec {
    fn deserialize<D: Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        d.deserialize_any(DecVisitor)
    }
}

macro_rules! mirror {
    ($wire:ident, $core:ident, $($v:ident),+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $wire { $($v),+ }

        impl From<$wire> for $core {
            fn from(v: $wire) -> Self {
                match v { $($wire::$v => Self::$v),+ }
            }
        }

        impl From<$core> for $wire {
            fn from(v: $core) -> Self {
                match v { $($core::$v => Self::$v),+ }
            }
        }
    };
}

mirror!(SecurityWire, SecurityLevel, StrongBox, Tee, Dev);
mirror!(ProvisioningWire, Provisioning, Rkp, Factory, Dev);
mirror!(FreshnessWire, Freshness, Fresh, Late);
mirror!(StatusWire, KeyStatus, Active, Retired, Suspended, Revoked);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusChangeWire {
    pub status: StatusWire,
    pub since_ns: Dec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyWire {
    pub key_id: Hex,
    pub pubkey: Hex,
    pub security: SecurityWire,
    pub provisioning: ProvisioningWire,
    pub freshness: FreshnessWire,
    pub patch: u32,
    pub enrolled_ns: Dec,
    pub active_until_ns: Dec,
    pub history: Vec<StatusChangeWire>,
    pub chain_serials: Vec<String>,
}

impl KeyWire {
    pub fn to_info(&self) -> Result<KeyInfo> {
        let malformed = |_| Error::Malformed("key");
        let history = KeyHistory::from_changes(
            self.history
                .iter()
                .map(|c| StatusChange {
                    status: c.status.into(),
                    since_ns: c.since_ns.0,
                })
                .collect(),
        )
        .map_err(malformed)?;
        let info = KeyInfo {
            key_id: KeyId::try_from(self.key_id.0.as_slice()).map_err(malformed)?,
            pubkey: self.pubkey.0.clone(),
            security: self.security.into(),
            provisioning: self.provisioning.into(),
            freshness: self.freshness.into(),
            patch: self.patch,
            enrolled_ns: self.enrolled_ns.0,
            active_until_ns: self.active_until_ns.0,
            history,
            chain_serials: self.chain_serials.clone(),
        };
        info.validate().map_err(malformed)?;
        Ok(info)
    }

    pub fn from_info(k: &KeyInfo) -> Self {
        Self {
            key_id: Hex(k.key_id.as_bytes().to_vec()),
            pubkey: Hex(k.pubkey.clone()),
            security: k.security.into(),
            provisioning: k.provisioning.into(),
            freshness: k.freshness.into(),
            patch: k.patch,
            enrolled_ns: Dec(k.enrolled_ns),
            active_until_ns: Dec(k.active_until_ns),
            history: k
                .history
                .changes()
                .iter()
                .map(|c| StatusChangeWire {
                    status: c.status.into(),
                    since_ns: Dec(c.since_ns),
                })
                .collect(),
            chain_serials: k.chain_serials.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertifiedKeyWire {
    pub key: KeyWire,
    pub smt_proof: Hex,
    pub certificate: Hex,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordWire {
    pub record_ix: Dec,
    pub record_id: String,
    pub claim: Hex,
    #[serde(default)]
    pub c2pa_claim: Hex,
    #[serde(default)]
    pub cose: Hex,
    pub anchored_ns: Dec,
    #[serde(default)]
    pub mmr_proof: Vec<Hex>,
    #[serde(default)]
    pub leaf_count: Dec,
    #[serde(default)]
    pub certificate: Hex,
    #[serde(default)]
    pub media_shard: u8,
}

impl RecordWire {
    pub fn record_id(&self) -> Result<RecordId> {
        let derived = RecordId::from_claim_cbor(&self.claim.0);
        match RecordId::parse(&self.record_id) {
            Ok(id) if id == derived => Ok(id),
            _ => Err(Error::Malformed("record id")),
        }
    }

    pub fn to_record(&self) -> Result<Record> {
        let record_id = self.record_id()?;
        let claim =
            WitnessClaim::from_cbor(&self.claim.0).map_err(|_| Error::Malformed("record claim"))?;
        Ok(Record {
            record_ix: self.record_ix.0,
            record_id,
            anchored_ns: self.anchored_ns.0,
            flags: witness_core::verdict::clock_flags(&claim, self.anchored_ns.0),
            claim,
        })
    }

    pub fn from_parts(record: &Record, claim_cbor: Vec<u8>) -> Self {
        Self {
            record_ix: Dec(record.record_ix),
            record_id: record.record_id.to_string(),
            claim: Hex(claim_cbor),
            anchored_ns: Dec(record.anchored_ns),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintsWire {
    pub grid: Hex,
    pub shapes: Vec<(u8, Hex)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait: Option<bool>,
}

impl FingerprintsWire {
    pub fn from_bundle(b: &FingerprintBundle) -> Self {
        Self {
            grid: Hex(b.grid_bytes().to_vec()),
            shapes: b
                .pdq()
                .iter()
                .map(|s| (s.tag, Hex(s.hash.to_vec())))
                .collect(),
            quality: Some(b.quality()),
            portrait: Some(b.portrait()),
        }
    }

    fn grid(&self) -> Option<[u16; GRID_CELLS]> {
        let bytes: &[u8; GRID_BYTES] = self.grid.0.as_slice().try_into().ok()?;
        let mut cells = [0u16; GRID_CELLS];
        for (cell, pair) in cells.iter_mut().zip(bytes.chunks_exact(2)) {
            *cell = u16::from_be_bytes([pair[0], pair[1]]);
        }
        Some(cells)
    }

    pub fn bundle_for(&self, claim: &WitnessClaim) -> Option<FingerprintBundle> {
        let grid = self.grid()?;
        let shapes = self
            .shapes
            .iter()
            .map(|(tag, hash)| {
                Some(PdqShape {
                    tag: *tag,
                    hash: hash.0.as_slice().try_into().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let portrait = self
            .portrait
            .unwrap_or_else(|| is_portrait(claim.dims.width, claim.dims.height));
        let qualities = self.quality.map_or(0..=MAX_PDQ_QUALITY, |q| q..=q);
        qualities
            .filter_map(|q| FingerprintBundle::new(shapes.clone(), q, grid, portrait).ok())
            .find(|b| b.fp_commit() == claim.fp_commit)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdqHitWire {
    pub ix: Dec,
    pub distance: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdqWire {
    pub hits: Vec<PdqHitWire>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fetched {
    pub keys: BTreeMap<Hex, Option<KeyWire>>,
    pub records: BTreeMap<String, Option<RecordWire>>,
    pub absent_ixs: Vec<Dec>,
    pub exact: Option<Vec<Dec>>,
    pub watermark: BTreeMap<Dec, Vec<Dec>>,
    pub pdq: Option<PdqWire>,
    pub derivatives: BTreeMap<Dec, Vec<Dec>>,
    pub fingerprints: BTreeMap<Dec, Option<FingerprintsWire>>,
    pub references: BTreeMap<String, Option<B64>>,
    pub index_pending: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckOptions {
    #[serde(default)]
    pub anchor: Hex,
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub dev_anchor: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pixels: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardInfoWire {
    pub shard: u8,
    pub canister: Hex,
    pub open: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardsWire {
    pub index: Vec<ShardInfoWire>,
    pub media: Vec<ShardInfoWire>,
    pub bundle: Hex,
}

#[cfg(test)]
mod tests;
