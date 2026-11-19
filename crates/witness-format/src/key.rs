use crate::cbor::{self, Decode, Decoder, Encode, Encoder, Error};
use crate::crypto::sha256;
use crate::evidence::SecurityLevel;
use crate::ids::KeyId;
use alloc::string::String;
use alloc::vec::Vec;

pub const MAX_HISTORY: usize = 64;
pub const MAX_CHAIN: usize = 8;
pub const MAX_SERIAL_HEX: usize = 64;
pub const MAX_PUBKEY: usize = 256;

code_enum! {
    pub enum KeyStatus {
        Active = 0,
        Retired = 1,
        Suspended = 2,
        Revoked = 3,
    }
}

code_enum! {
    pub enum Provisioning {
        Rkp = 0,
        Factory = 1,
        Dev = 2,
    }
}

code_enum! {
    pub enum Freshness {
        Fresh = 0,
        Late = 1,
    }
}

impl KeyStatus {
    pub const fn is_trusted(self) -> bool {
        matches!(self, Self::Active | Self::Retired)
    }

    pub const fn is_distrusted(self) -> bool {
        matches!(self, Self::Suspended | Self::Revoked)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusChange {
    pub status: KeyStatus,
    pub since_ns: u64,
}

impl Encode for StatusChange {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.put(&(self.status, self.since_ns));
    }
}

impl<'a> Decode<'a> for StatusChange {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let (status, since_ns) = <(KeyStatus, u64)>::decode(d)?;
        Ok(Self { status, since_ns })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyHistory(Vec<StatusChange>);

impl KeyHistory {
    pub fn new(enrolled_ns: u64) -> Self {
        Self(alloc::vec![StatusChange {
            status: KeyStatus::Active,
            since_ns: enrolled_ns
        }])
    }

    pub fn from_changes(changes: Vec<StatusChange>) -> Result<Self, Error> {
        if changes.is_empty() || changes.len() > MAX_HISTORY {
            return Err(Error::Length);
        }
        let ordered = changes.windows(2).all(|w| {
            w[0].since_ns < w[1].since_ns
                && w[0].status != w[1].status
                && w[0].status != KeyStatus::Revoked
        });
        if ordered {
            Ok(Self(changes))
        } else {
            Err(Error::Invalid)
        }
    }

    pub fn changes(&self) -> &[StatusChange] {
        &self.0
    }

    pub fn current(&self) -> KeyStatus {
        self.0[self.0.len() - 1].status
    }

    pub fn push(&mut self, change: StatusChange) -> Result<(), Error> {
        let last = self.0[self.0.len() - 1];
        if self.0.len() >= MAX_HISTORY {
            return Err(Error::Length);
        }
        if change.since_ns <= last.since_ns
            || change.status == last.status
            || last.status == KeyStatus::Revoked
        {
            return Err(Error::Invalid);
        }
        self.0.push(change);
        Ok(())
    }

    pub fn status_at(&self, ns: u64) -> Option<KeyStatus> {
        let i = self.0.partition_point(|c| c.since_ns <= ns);
        i.checked_sub(1).map(|i| self.0[i].status)
    }

    pub fn active_at(&self, ns: u64) -> bool {
        self.status_at(ns).is_some_and(KeyStatus::is_trusted)
    }

    pub fn revoked_after(&self, ns: u64) -> bool {
        let i = self.0.partition_point(|c| c.since_ns <= ns);
        self.0[i..].iter().any(|c| c.status.is_distrusted())
    }

    pub fn first_distrusted_after(&self, ns: u64) -> Option<StatusChange> {
        let i = self.0.partition_point(|c| c.since_ns <= ns);
        self.0[i..]
            .iter()
            .copied()
            .find(|c| c.status.is_distrusted())
    }
}

impl Encode for KeyHistory {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.put(&self.0);
    }
}

impl<'a> Decode<'a> for KeyHistory {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        Self::from_changes(Vec::decode(d)?)
    }
}

pub fn is_normalized_serial_hex(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= MAX_SERIAL_HEX
        && (b == b"0" || b[0] != b'0')
        && b.iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn normalize_serial_hex(magnitude: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let start = magnitude
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(magnitude.len());
    let digits = &magnitude[start..];
    let mut out = String::with_capacity(digits.len() * 2 + 1);
    for (i, &b) in digits.iter().enumerate() {
        if i > 0 || b >> 4 != 0 {
            out.push(char::from(HEX[usize::from(b >> 4)]));
        }
        out.push(char::from(HEX[usize::from(b & 15)]));
    }
    if out.is_empty() {
        out.push('0');
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyInfo {
    pub key_id: KeyId,
    pub pubkey: Vec<u8>,
    pub security: SecurityLevel,
    pub provisioning: Provisioning,
    pub freshness: Freshness,
    pub patch: u32,
    pub enrolled_ns: u64,
    pub active_until_ns: u64,
    pub history: KeyHistory,
    pub chain_serials: Vec<String>,
}

impl KeyInfo {
    pub const VERSION: u64 = 1;

    pub fn validate(&self) -> Result<(), Error> {
        if self.pubkey.is_empty() || self.pubkey.len() > MAX_PUBKEY {
            return Err(Error::Length);
        }
        if KeyId::from_spki(&self.pubkey) != self.key_id {
            return Err(Error::Invalid);
        }
        if self.active_until_ns < self.enrolled_ns {
            return Err(Error::Range);
        }
        if self.chain_serials.len() > MAX_CHAIN {
            return Err(Error::Length);
        }
        if !self
            .chain_serials
            .iter()
            .all(|s| is_normalized_serial_hex(s))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }

    pub fn is_dev(&self) -> bool {
        self.provisioning == Provisioning::Dev || self.security == SecurityLevel::Dev
    }

    pub fn accepts_original_at_ms(&self, wall_ms: u64) -> bool {
        u128::from(wall_ms) * 1_000_000 <= u128::from(self.active_until_ns)
    }

    pub fn leaf_hash(&self) -> [u8; 32] {
        sha256(&cbor::to_vec(self))
    }
}

impl Encode for KeyInfo {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(11);
        e.field(0, &Self::VERSION);
        e.field(1, &self.key_id);
        e.field(2, self.pubkey.as_slice());
        e.field(3, &self.security);
        e.field(4, &self.provisioning);
        e.field(5, &self.freshness);
        e.field(6, &self.patch);
        e.field(7, &self.enrolled_ns);
        e.field(8, &self.active_until_ns);
        e.field(9, &self.history);
        e.field(10, &self.chain_serials);
    }
}

impl<'a> Decode<'a> for KeyInfo {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, Self::VERSION)?;
        let info = Self {
            key_id: f.req(d, 1)?,
            pubkey: f.req::<&[u8]>(d, 2)?.to_vec(),
            security: f.req(d, 3)?,
            provisioning: f.req(d, 4)?,
            freshness: f.req(d, 5)?,
            patch: f.req(d, 6)?,
            enrolled_ns: f.req(d, 7)?,
            active_until_ns: f.req(d, 8)?,
            history: f.req(d, 9)?,
            chain_serials: f.req(d, 10)?,
        };
        f.finish(d)?;
        info.validate()?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests;
