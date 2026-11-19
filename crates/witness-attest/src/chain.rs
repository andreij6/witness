use crate::Error;
use crate::roots::{self, RootKind, TrustAnchor};
use crate::x509::Certificate;
use witness_format::key::MAX_CHAIN;

pub const MIN_CHAIN: usize = 3;

pub fn check_length(len: usize) -> Result<(), Error> {
    if (MIN_CHAIN..=MAX_CHAIN).contains(&len) {
        Ok(())
    } else {
        Err(Error::ChainLength)
    }
}

pub fn root_kind(
    certs: &[Certificate<'_>],
    anchors: &[TrustAnchor<'_>],
) -> Result<RootKind, Error> {
    let root = certs.last().ok_or(Error::ChainLength)?;
    roots::find(anchors, root.spki)
        .map(|a| a.kind)
        .ok_or(Error::UntrustedRoot)
}

pub fn check_issuers(certs: &[Certificate<'_>]) -> Result<(), Error> {
    for (below, issuer) in certs.iter().skip(1).enumerate() {
        let bc = issuer
            .basic_constraints
            .filter(|b| b.ca)
            .ok_or(Error::NotCa)?;
        if bc.path_len.is_some_and(|p| (p as usize) < below) {
            return Err(Error::PathLen);
        }
        if issuer.key_cert_sign == Some(false) {
            return Err(Error::KeyUsage);
        }
    }
    Ok(())
}

pub fn check_validity(
    certs: &[Certificate<'_>],
    exempt_issuers: bool,
    now_secs: i64,
) -> Result<(), Error> {
    let issuers = certs.len().saturating_sub(1);
    let checked = if exempt_issuers { 1 } else { issuers };
    if certs
        .iter()
        .take(checked.max(1))
        .all(|c| c.valid_at(now_secs))
    {
        Ok(())
    } else {
        Err(Error::Validity)
    }
}

pub fn verify_signatures(certs: &[Certificate<'_>]) -> Result<(), Error> {
    certs
        .windows(2)
        .try_for_each(|w| w[0].verify_signed_by(&w[1].public_key()?))
}

#[cfg(test)]
mod tests;
