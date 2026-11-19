use crate::cbor::{Decode, Decoder, Encode, Encoder, Error};
use crate::crypto::{ct_eq, domain, hash_ds};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeEvidence {
    pub wall_ms: u64,
    pub gnss_ms: Option<u64>,
    pub elapsed_ns: u64,
    pub boot_count: u32,
}

impl TimeEvidence {
    pub const fn clock_skew_ms(&self) -> Option<u64> {
        match self.gnss_ms {
            Some(g) => Some(self.wall_ms.abs_diff(g)),
            None => None,
        }
    }
}

impl Encode for TimeEvidence {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(3 + usize::from(self.gnss_ms.is_some()));
        e.field(0, &self.wall_ms);
        e.opt_field(1, self.gnss_ms.as_ref());
        e.field(2, &self.elapsed_ns);
        e.field(3, &self.boot_count);
    }
}

impl<'a> Decode<'a> for TimeEvidence {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let t = Self {
            wall_ms: f.req(d, 0)?,
            gnss_ms: f.opt(d, 1)?,
            elapsed_ns: f.req(d, 2)?,
            boot_count: f.req(d, 3)?,
        };
        f.finish(d)?;
        Ok(t)
    }
}

code_enum! {
    pub enum Confidence {
        High = 0,
        Medium = 1,
        Low = 2,
    }
}

bitset! {
    pub struct LocationReasons: u16 {
        const RAW_GNSS_PASSED = 0;
        const FUSED_FIX = 1;
        const NO_FIX = 2;
        const WEAK_SIGNAL = 3;
        const CN0_ANOMALY = 4;
        const AGC_JUMP = 5;
        const PSEUDORANGE_RESIDUAL = 6;
        const CLOCK_DRIFT = 7;
        const MOCK_LOCATION = 8;
        const MCC_MISMATCH = 9;
        const INDOOR = 10;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocationEvidence {
    pub commit: Option<[u8; 32]>,
    pub confidence: Confidence,
    pub reasons: LocationReasons,
}

impl Encode for LocationEvidence {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(2 + usize::from(self.commit.is_some()));
        e.opt_field(0, self.commit.as_ref());
        e.field(1, &self.confidence);
        e.field(2, &self.reasons);
    }
}

impl<'a> Decode<'a> for LocationEvidence {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let l = Self {
            commit: f.opt(d, 0)?,
            confidence: f.req(d, 1)?,
            reasons: f.req(d, 2)?,
        };
        f.finish(d)?;
        Ok(l)
    }
}

pub const LAT_E7_MAX: i32 = 900_000_000;
pub const LON_E7_MAX: i32 = 1_800_000_000;

pub fn location_commit(salt: &[u8; 32], lat_e7: i32, lon_e7: i32, acc_cm: u32) -> [u8; 32] {
    hash_ds(
        domain::LOCATION,
        &[
            salt,
            &lat_e7.to_be_bytes(),
            &lon_e7.to_be_bytes(),
            &acc_cm.to_be_bytes(),
        ],
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LocationOpening {
    pub salt: [u8; 32],
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub acc_cm: u32,
}

impl core::fmt::Debug for LocationOpening {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LocationOpening").finish_non_exhaustive()
    }
}

impl LocationOpening {
    pub const VERSION: u64 = 1;

    pub const fn in_range(&self) -> bool {
        self.lat_e7 >= -LAT_E7_MAX
            && self.lat_e7 <= LAT_E7_MAX
            && self.lon_e7 >= -LON_E7_MAX
            && self.lon_e7 <= LON_E7_MAX
    }

    pub fn commit(&self) -> [u8; 32] {
        location_commit(&self.salt, self.lat_e7, self.lon_e7, self.acc_cm)
    }

    pub fn verify(&self, commit: &[u8; 32]) -> bool {
        let ok = ct_eq(&self.commit(), commit);
        ok & self.in_range()
    }
}

impl Encode for LocationOpening {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(5);
        e.field(0, &Self::VERSION);
        e.field(1, &self.salt);
        e.field(2, &self.lat_e7);
        e.field(3, &self.lon_e7);
        e.field(4, &self.acc_cm);
    }
}

impl<'a> Decode<'a> for LocationOpening {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, Self::VERSION)?;
        let o = Self {
            salt: f.req(d, 1)?,
            lat_e7: f.req(d, 2)?,
            lon_e7: f.req(d, 3)?,
            acc_cm: f.req(d, 4)?,
        };
        f.finish(d)?;
        if o.in_range() {
            Ok(o)
        } else {
            Err(Error::Range)
        }
    }
}

code_enum! {
    pub enum SecurityLevel {
        StrongBox = 0,
        Tee = 1,
        Dev = 2,
    }
}

code_enum! {
    pub enum BootState {
        Verified = 0,
        SelfSigned = 1,
        Unverified = 2,
        Failed = 3,
    }
}

bitset! {
    pub struct GnssChecks: u8 {
        const RAW_AVAILABLE = 0;
        const CN0_CONSISTENT = 1;
        const AGC_STABLE = 2;
        const PSEUDORANGE_CONSISTENT = 3;
        const CLOCK_CONTINUOUS = 4;
        const MCC_MATCH = 5;
        const GNSS_TIME = 6;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checks {
    pub key_security: SecurityLevel,
    pub boot: BootState,
    pub device_locked: bool,
    pub mock_location: bool,
    pub gnss: GnssChecks,
    pub recapture: u8,
}

impl Checks {
    const SECURITY_SHIFT: u32 = 0;
    const BOOT_SHIFT: u32 = 2;
    const LOCKED_BIT: u32 = 1 << 4;
    const MOCK_BIT: u32 = 1 << 5;
    const GNSS_SHIFT: u32 = 8;
    const RECAPTURE_SHIFT: u32 = 16;
    const RESERVED: u32 = 0xff00_00c0;

    pub const fn to_bits(&self) -> u32 {
        ((self.key_security.code() as u32) << Self::SECURITY_SHIFT)
            | ((self.boot.code() as u32) << Self::BOOT_SHIFT)
            | if self.device_locked {
                Self::LOCKED_BIT
            } else {
                0
            }
            | if self.mock_location {
                Self::MOCK_BIT
            } else {
                0
            }
            | ((self.gnss.bits() as u32) << Self::GNSS_SHIFT)
            | ((self.recapture as u32) << Self::RECAPTURE_SHIFT)
    }

    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & Self::RESERVED != 0 {
            return None;
        }
        let Some(key_security) = SecurityLevel::from_code((bits & 3) as u8) else {
            return None;
        };
        let Some(boot) = BootState::from_code(((bits >> Self::BOOT_SHIFT) & 3) as u8) else {
            return None;
        };
        let Some(gnss) = GnssChecks::from_bits((bits >> Self::GNSS_SHIFT) as u8) else {
            return None;
        };
        Some(Self {
            key_security,
            boot,
            device_locked: bits & Self::LOCKED_BIT != 0,
            mock_location: bits & Self::MOCK_BIT != 0,
            gnss,
            recapture: (bits >> Self::RECAPTURE_SHIFT) as u8,
        })
    }

    pub const fn boot_trusted(&self) -> bool {
        matches!(self.boot, BootState::Verified) && self.device_locked
    }
}

impl Encode for Checks {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.u64(u64::from(self.to_bits()));
    }
}

impl<'a> Decode<'a> for Checks {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        u32::try_from(d.u64()?)
            .ok()
            .and_then(Self::from_bits)
            .ok_or(Error::Range)
    }
}

#[cfg(test)]
mod tests;
