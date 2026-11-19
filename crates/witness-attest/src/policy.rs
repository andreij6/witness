use crate::Error;
use crate::chain;
use crate::challenge::{FRESH_LEN, LATE_LEN, SECRET_LEN, fresh_issued_ns, late_challenge};
use crate::keydesc::{
    ALGORITHM_EC, AppId, AuthList, CURVE_P256, KeyDescription, KmSecurity, ORIGIN_GENERATED,
    PURPOSE_SIGN, VerifiedBoot,
};
use crate::revocation::RevocationList;
use crate::roots::{ANCHORS, RootKind, TrustAnchor};
use crate::time::{NS_PER_MS, ns_to_secs};
use crate::x509::Certificate;
use alloc::string::String;
use alloc::vec::Vec;
use witness_format::crypto::{ct_eq, sha256};
use witness_format::{
    BootState, Freshness, KeyHistory, KeyId, KeyInfo, Provisioning, SecurityLevel,
};

pub const MINUTE_NS: u64 = 60 * 1_000_000_000;
pub const HOUR_NS: u64 = 60 * MINUTE_NS;
pub const DAY_NS: u64 = 24 * HOUR_NS;
pub const WITNESS_PACKAGE: &[u8] = b"app.witness";
pub const DEFAULT_MIN_PATCH: u32 = 202_501;
pub const DEFAULT_MIN_VERSION: u64 = 1;
pub const MAX_INSTALL_NONCE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy<'a> {
    pub anchors: &'a [TrustAnchor<'a>],
    pub package: &'a [u8],
    pub min_version: u64,
    pub release_digests: &'a [[u8; 32]],
    pub min_patch: u32,
    pub fresh_ttl_ns: u64,
    pub late_ttl_ns: u64,
    pub max_skew_ns: u64,
    pub active_ns: u64,
}

impl<'a> Policy<'a> {
    pub const fn witness(release_digests: &'a [[u8; 32]]) -> Self {
        Self {
            anchors: ANCHORS,
            package: WITNESS_PACKAGE,
            min_version: DEFAULT_MIN_VERSION,
            release_digests,
            min_patch: DEFAULT_MIN_PATCH,
            fresh_ttl_ns: HOUR_NS,
            late_ttl_ns: 7 * DAY_NS,
            max_skew_ns: 10 * MINUTE_NS,
            active_ns: 60 * DAY_NS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub now_ns: u64,
    pub challenge_secret: &'a [u8; SECRET_LEN],
    pub revocations: &'a RevocationList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LateOpening<'a> {
    pub install_nonce: &'a [u8],
    pub wall_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Enrollment<'a, C> {
    pub chain: &'a [C],
    pub key_id: Option<KeyId>,
    pub late: Option<LateOpening<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attested {
    pub key_id: KeyId,
    pub pubkey: Vec<u8>,
    pub security: SecurityLevel,
    pub provisioning: Provisioning,
    pub freshness: Freshness,
    pub patch: u32,
    pub boot: BootState,
    pub device_locked: bool,
    pub created_ms: Option<u64>,
    pub attestation_version: u32,
    pub keymint_version: u32,
    pub chain_serials: Vec<String>,
    pub quota_key: [u8; 32],
}

impl Attested {
    pub fn key_info(&self, enrolled_ns: u64, active_ns: u64) -> KeyInfo {
        KeyInfo {
            key_id: self.key_id,
            pubkey: self.pubkey.clone(),
            security: self.security,
            provisioning: self.provisioning,
            freshness: self.freshness,
            patch: self.patch,
            enrolled_ns,
            active_until_ns: enrolled_ns.saturating_add(active_ns),
            history: KeyHistory::new(enrolled_ns),
            chain_serials: self.chain_serials.clone(),
        }
    }
}

pub fn attestation_index(certs: &[Certificate<'_>], root: RootKind) -> Result<usize, Error> {
    let kd = certs
        .iter()
        .rposition(|c| c.key_description.is_some())
        .ok_or(Error::MissingKeyDescription)?;
    let below_dev_root = match root {
        RootKind::Dev => certs.len().saturating_sub(1),
        _ => certs.len(),
    };
    let placed_after_provisioning = certs[..below_dev_root]
        .iter()
        .rposition(|c| c.provisioning_info)
        .is_none_or(|p| p.checked_sub(1) == Some(kd));
    if kd == 0 && placed_after_provisioning {
        Ok(kd)
    } else {
        Err(Error::ExtensionPlacement)
    }
}

pub fn provisioning(certs: &[Certificate<'_>], root: RootKind) -> Provisioning {
    match root {
        RootKind::Dev => Provisioning::Dev,
        RootKind::GoogleEc => Provisioning::Rkp,
        RootKind::GoogleRsa if certs.iter().any(|c| c.provisioning_info) => Provisioning::Rkp,
        RootKind::GoogleRsa => Provisioning::Factory,
    }
}

const fn security_level(level: KmSecurity) -> Option<SecurityLevel> {
    match level {
        KmSecurity::Tee => Some(SecurityLevel::Tee),
        KmSecurity::StrongBox => Some(SecurityLevel::StrongBox),
        KmSecurity::Software => None,
    }
}

const fn boot_state(state: VerifiedBoot) -> BootState {
    match state {
        VerifiedBoot::Verified => BootState::Verified,
        VerifiedBoot::SelfSigned => BootState::SelfSigned,
        VerifiedBoot::Unverified => BootState::Unverified,
        VerifiedBoot::Failed => BootState::Failed,
    }
}

struct Lists<'k, 'a> {
    kd: &'k KeyDescription<'a>,
    dev: bool,
}

impl<'a> Lists<'_, 'a> {
    fn enforced<T>(&self, f: impl Fn(&AuthList<'a>) -> Option<T>) -> Option<T> {
        f(&self.kd.hardware).or_else(|| if self.dev { f(&self.kd.software) } else { None })
    }

    fn any<T>(&self, f: impl Fn(&AuthList<'a>) -> Option<T>) -> Option<T> {
        f(&self.kd.hardware).or_else(|| f(&self.kd.software))
    }

    fn security(&self) -> Result<SecurityLevel, Error> {
        let attestation = security_level(self.kd.attestation_security);
        match (attestation, security_level(self.kd.keymint_security)) {
            (Some(_), Some(level)) => Ok(level),
            (_, level) if self.dev => Ok(level.unwrap_or(SecurityLevel::Dev)),
            _ => Err(Error::SecurityLevel),
        }
    }

    fn boot(&self) -> Result<(BootState, bool), Error> {
        let rot = self.enforced(|l| l.root_of_trust);
        if self.dev {
            return Ok(rot.map_or((BootState::Unverified, false), |r| {
                (boot_state(r.state), r.device_locked)
            }));
        }
        let rot = rot.ok_or(Error::BootState)?;
        if rot.state != VerifiedBoot::Verified {
            return Err(Error::BootState);
        }
        if !rot.device_locked {
            return Err(Error::DeviceLocked);
        }
        Ok((BootState::Verified, true))
    }

    fn key(&self, leaf: &Certificate<'_>) -> Result<(), Error> {
        if self.enforced(|l| l.origin) != Some(ORIGIN_GENERATED) {
            return Err(Error::Origin);
        }
        let signs = |l: &AuthList<'_>| l.has_purpose(PURPOSE_SIGN);
        if !(signs(&self.kd.hardware) || self.dev && signs(&self.kd.software)) {
            return Err(Error::Purpose);
        }
        if self.enforced(|l| l.algorithm) != Some(ALGORITHM_EC) || !leaf.public_key()?.is_p256() {
            return Err(Error::Algorithm);
        }
        if self.enforced(|l| l.ec_curve) != Some(CURVE_P256) {
            return Err(Error::Curve);
        }
        Ok(())
    }

    fn patch(&self, min: u32) -> Result<u32, Error> {
        self.enforced(|l| l.os_patch_level)
            .filter(|&p| p >= min)
            .ok_or(Error::PatchLevel)
    }

    fn app(&self, policy: &Policy<'_>) -> Result<(), Error> {
        let raw = self
            .any(|l| l.attestation_application_id)
            .ok_or(Error::Package)?;
        let app = AppId::parse(raw)?;
        let version = app
            .packages
            .iter()
            .filter(|(name, _)| *name == policy.package)
            .map(|&(_, v)| v)
            .max()
            .ok_or(Error::Package)?;
        if version < policy.min_version {
            return Err(Error::AppVersion);
        }
        let signed = app
            .digests
            .iter()
            .any(|d| policy.release_digests.iter().any(|r| r.as_slice() == *d));
        if signed {
            Ok(())
        } else {
            Err(Error::SignerDigest)
        }
    }
}

fn within_window(t_ms: u64, now_ns: u64, ttl_ns: u64, skew_ns: u64) -> bool {
    let t = u128::from(t_ms) * u128::from(NS_PER_MS);
    let now = u128::from(now_ns);
    t <= now + u128::from(skew_ns) && now.saturating_sub(t) <= u128::from(ttl_ns)
}

fn freshness(
    kd: &KeyDescription<'_>,
    leaf: &Certificate<'_>,
    late: Option<LateOpening<'_>>,
    policy: &Policy<'_>,
    ctx: &Context<'_>,
) -> Result<(Freshness, Option<u64>), Error> {
    let created_ms = kd.hardware.creation_ms.or(kd.software.creation_ms);
    match kd.challenge.len() {
        FRESH_LEN => {
            let issued = fresh_issued_ns(ctx.challenge_secret, kd.challenge)
                .filter(|&i| i <= ctx.now_ns)
                .ok_or(Error::Challenge)?;
            if ctx.now_ns - issued > policy.fresh_ttl_ns {
                return Err(Error::ChallengeExpired);
            }
            Ok((Freshness::Fresh, created_ms))
        }
        LATE_LEN => {
            let open = late
                .filter(|o| (1..=MAX_INSTALL_NONCE).contains(&o.install_nonce.len()))
                .ok_or(Error::Challenge)?;
            if !ct_eq(
                &late_challenge(open.install_nonce, open.wall_ms),
                kd.challenge,
            ) {
                return Err(Error::Challenge);
            }
            let created = match created_ms {
                Some(ms) => ms,
                None => u64::try_from(leaf.not_before)
                    .ok()
                    .and_then(|s| s.checked_mul(1000))
                    .ok_or(Error::CreationTime)?,
            };
            let ok = |t| within_window(t, ctx.now_ns, policy.late_ttl_ns, policy.max_skew_ns);
            if ok(created) && ok(open.wall_ms) {
                Ok((Freshness::Late, Some(created)))
            } else {
                Err(Error::CreationTime)
            }
        }
        _ => Err(Error::Challenge),
    }
}

pub fn verify<C: AsRef<[u8]>>(
    enrollment: &Enrollment<'_, C>,
    policy: &Policy<'_>,
    ctx: &Context<'_>,
) -> Result<Attested, Error> {
    chain::check_length(enrollment.chain.len())?;
    let certs = enrollment
        .chain
        .iter()
        .map(|c| Certificate::parse(c.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;
    let leaf = &certs[0];
    let key_id = KeyId::from_spki(leaf.spki);
    if enrollment.key_id.is_some_and(|k| k != key_id) {
        return Err(Error::KeyMismatch);
    }
    let root = chain::root_kind(&certs, policy.anchors)?;
    chain::check_issuers(&certs)?;
    let kd_index = attestation_index(&certs, root)?;
    let kd_bytes = certs[kd_index]
        .key_description
        .ok_or(Error::MissingKeyDescription)?;
    let kd = KeyDescription::parse(kd_bytes)?;
    let provisioning = provisioning(&certs, root);
    let lists = Lists {
        kd: &kd,
        dev: root == RootKind::Dev,
    };
    let security = lists.security()?;
    let (boot, device_locked) = lists.boot()?;
    lists.key(leaf)?;
    let patch = lists.patch(policy.min_patch)?;
    lists.app(policy)?;
    let (freshness, created_ms) = freshness(&kd, leaf, enrollment.late, policy, ctx)?;
    chain::check_validity(
        &certs,
        provisioning != Provisioning::Rkp,
        ns_to_secs(ctx.now_ns),
    )?;
    if let Some(status) = certs.iter().find_map(|c| ctx.revocations.status(&c.serial)) {
        return Err(Error::Revoked(status));
    }
    chain::verify_signatures(&certs)?;
    Ok(Attested {
        key_id,
        pubkey: leaf.spki.to_vec(),
        security,
        provisioning,
        freshness,
        patch,
        boot,
        device_locked,
        created_ms,
        attestation_version: kd.attestation_version,
        keymint_version: kd.keymint_version,
        chain_serials: certs.iter().map(|c| c.serial.to_hex()).collect(),
        quota_key: sha256(certs[1].der),
    })
}

#[cfg(test)]
mod tests;
