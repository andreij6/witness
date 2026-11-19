use crate::error::{Error, Result};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use ic_certification::{Certificate, HashTree, LookupResult, SubtreeLookupResult};
use serde_bytes::ByteBuf;

pub const DER_PREFIX: [u8; 37] = [
    0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c, 0x05,
    0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c, 0x05, 0x03,
    0x02, 0x01, 0x03, 0x61, 0x00,
];
pub const RAW_KEY_LEN: usize = 96;
pub const DOMAIN: &[u8] = b"\x0dic-state-root";
pub const DEFAULT_MAX_AGE_NS: u64 = 300_000_000_000;
pub const IC_ROOT_KEY_DER: [u8; 133] = [
    0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c, 0x05,
    0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c, 0x05, 0x03,
    0x02, 0x01, 0x03, 0x61, 0x00, 0x81, 0x4c, 0x0e, 0x6e, 0xc7, 0x1f, 0xab, 0x58, 0x3b, 0x08, 0xbd,
    0x81, 0x37, 0x3c, 0x25, 0x5c, 0x3c, 0x37, 0x1b, 0x2e, 0x84, 0x86, 0x3c, 0x98, 0xa4, 0xf1, 0xe0,
    0x8b, 0x74, 0x23, 0x5d, 0x14, 0xfb, 0x5d, 0x9c, 0x0c, 0xd5, 0x46, 0xd9, 0x68, 0x5f, 0x91, 0x3a,
    0x0c, 0x0b, 0x2c, 0xc5, 0x34, 0x15, 0x83, 0xbf, 0x4b, 0x43, 0x92, 0xe4, 0x67, 0xdb, 0x96, 0xd6,
    0x5b, 0x9b, 0xb4, 0xcb, 0x71, 0x71, 0x12, 0xf8, 0x47, 0x2e, 0x0d, 0x5a, 0x4d, 0x14, 0x50, 0x5f,
    0xfd, 0x74, 0x84, 0xb0, 0x12, 0x91, 0x09, 0x1c, 0x5f, 0x87, 0xb9, 0x88, 0x83, 0x46, 0x3f, 0x98,
    0x09, 0x1a, 0x0b, 0xaa, 0xae,
];

type Cert = Certificate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certified {
    pub data: Vec<u8>,
    pub time_ns: u64,
}

pub fn raw_key(der: &[u8]) -> Result<&[u8]> {
    der.strip_prefix(DER_PREFIX.as_slice())
        .filter(|k| k.len() == RAW_KEY_LEN)
        .ok_or(Error::Certificate("root key"))
}

fn parse(bytes: &[u8]) -> Result<Cert> {
    serde_cbor::from_slice(bytes).map_err(|_| Error::Certificate("cbor"))
}

fn found<'t>(r: LookupResult<'t>, what: &'static str) -> Result<&'t [u8]> {
    match r {
        LookupResult::Found(v) => Ok(v),
        _ => Err(Error::Certificate(what)),
    }
}

fn check_signature(cert: &Cert, key_der: &[u8]) -> Result<()> {
    let mut msg = Vec::with_capacity(DOMAIN.len() + 32);
    msg.extend_from_slice(DOMAIN);
    msg.extend_from_slice(&cert.tree.digest());
    ic_verify_bls_signature::verify_bls_signature(&cert.signature, &msg, raw_key(key_der)?)
        .map_err(|_| Error::Certificate("signature"))
}

fn ranges_blob(tree: &HashTree, subnet: &[u8], canister: &[u8]) -> Result<Vec<u8>> {
    match tree.lookup_subtree([b"canister_ranges".as_slice(), subnet]) {
        SubtreeLookupResult::Found(shards) => {
            let mut labels: Vec<Vec<u8>> = shards
                .list_paths()
                .into_iter()
                .filter_map(|mut p| p.pop().map(|l| l.as_bytes().to_vec()))
                .collect();
            labels.sort_unstable();
            let split = labels.partition_point(|l| l.as_slice() <= canister);
            let shard = split
                .checked_sub(1)
                .map(|i| &labels[i])
                .ok_or(Error::Certificate("canister outside delegation"))?;
            found(shards.lookup_path([shard.as_slice()]), "canister ranges").map(<[u8]>::to_vec)
        }
        _ => found(
            tree.lookup_path([b"subnet".as_slice(), subnet, b"canister_ranges"]),
            "canister ranges",
        )
        .map(<[u8]>::to_vec),
    }
}

pub const MAX_PRINCIPAL_LEN: usize = 29;

fn principal(bytes: &[u8]) -> Result<(usize, &[u8])> {
    if bytes.len() <= MAX_PRINCIPAL_LEN {
        Ok((bytes.len(), bytes))
    } else {
        Err(Error::Certificate("principal"))
    }
}

pub fn in_ranges(tree: &HashTree, subnet: &[u8], canister: &[u8]) -> Result<bool> {
    let blob = ranges_blob(tree, subnet, canister)?;
    let ranges: Vec<(ByteBuf, ByteBuf)> =
        serde_cbor::from_slice(&blob).map_err(|_| Error::Certificate("canister ranges"))?;
    let id = principal(canister)?;
    for (lo, hi) in &ranges {
        if principal(lo)? <= id && id <= principal(hi)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn signing_key<'k>(cert: &Cert, root_key: &'k [u8], canister: &[u8]) -> Result<Cow<'k, [u8]>> {
    let Some(d) = &cert.delegation else {
        return Ok(Cow::Borrowed(root_key));
    };
    let inner = parse(&d.certificate)?;
    if inner.delegation.is_some() {
        return Err(Error::Certificate("nested delegation"));
    }
    check_signature(&inner, root_key)?;
    if !in_ranges(&inner.tree, &d.subnet_id, canister)? {
        return Err(Error::Certificate("canister outside delegation"));
    }
    found(
        inner
            .tree
            .lookup_path([b"subnet".as_slice(), &d.subnet_id, b"public_key"]),
        "subnet key",
    )
    .map(|k| Cow::Owned(k.to_vec()))
}

pub fn leb128(bytes: &[u8]) -> Result<u64> {
    let mut value = 0u64;
    for (i, &b) in bytes.iter().enumerate() {
        let shift = 7 * i as u32;
        let part = u64::from(b & 0x7f);
        if shift >= 64 || (shift > 0 && part >> (64 - shift) != 0) {
            return Err(Error::Certificate("time"));
        }
        value |= part << shift;
        if b & 0x80 == 0 {
            return if i + 1 == bytes.len() {
                Ok(value)
            } else {
                Err(Error::Certificate("time"))
            };
        }
    }
    Err(Error::Certificate("time"))
}

pub fn fresh(time_ns: u64, now_ns: u64, max_age_ns: u64) -> Result<()> {
    if max_age_ns == 0 || now_ns.abs_diff(time_ns) <= max_age_ns {
        Ok(())
    } else {
        Err(Error::Certificate("stale"))
    }
}

pub fn verify(
    certificate: &[u8],
    root_key: &[u8],
    canister: &[u8],
    now_ns: u64,
    max_age_ns: u64,
) -> Result<Certified> {
    let cert = parse(certificate)?;
    let key = signing_key(&cert, root_key, canister)?;
    check_signature(&cert, &key)?;
    let time_ns = leb128(found(cert.tree.lookup_path([b"time"]), "time")?)?;
    fresh(time_ns, now_ns, max_age_ns)?;
    let data = found(
        cert.tree
            .lookup_path([b"canister".as_slice(), canister, b"certified_data"]),
        "certified data",
    )?;
    Ok(Certified {
        data: data.to_vec(),
        time_ns,
    })
}

#[cfg(test)]
mod tests;
