use crate::options::Options;
use crate::oracle::Oracle;
use crate::verdict::{Evidence, Notes, Reason, Reference, Verdict};
use witness_c2pa::{Credentials, Error, WitnessManifest, data_hash, verify_jpeg};
use witness_format::crypto::sha256;
use witness_format::{KeyInfo, Outcome, RecordId, VerdictFlags};
use witness_image::{Format, sniff};

pub(crate) enum Path {
    Decided(Box<Verdict>),
    Continue {
        asset_hash: [u8; 32],
        skip_exact: bool,
    },
}

fn asset_hash(input: &[u8]) -> [u8; 32] {
    data_hash(input).unwrap_or_else(|_| sha256(input))
}

fn structural(e: &Error) -> bool {
    matches!(
        e,
        Error::NotJpeg | Error::MalformedJpeg | Error::MalformedSegments
    )
}

fn signer_key<O: Oracle + ?Sized>(oracle: &O, m: &WitnessManifest) -> Option<KeyInfo> {
    let key_id = m.seal.claim.key.key_id;
    oracle
        .get_key(&key_id)
        .filter(|k| k.key_id == key_id && k.pubkey == m.seal.signer_spki)
}

pub(crate) fn check<O: Oracle + ?Sized>(
    input: &[u8],
    oracle: &O,
    options: &Options<'_>,
    notes: &mut Notes,
) -> Path {
    if sniff(input) != Some(Format::Jpeg) {
        return Path::Continue {
            asset_hash: sha256(input),
            skip_exact: false,
        };
    }
    let report = match verify_jpeg(input, options.anchor, options.at_s) {
        Ok(report) => report,
        Err(e) => {
            if !structural(&e) {
                notes.reason(Reason::UnverifiedCredentials);
            }
            return Path::Continue {
                asset_hash: asset_hash(input),
                skip_exact: false,
            };
        }
    };
    let proceed = |skip_exact| Path::Continue {
        asset_hash: report.data_hash,
        skip_exact,
    };
    let manifest = match &report.credentials {
        Credentials::Absent => return proceed(false),
        Credentials::Other => {
            notes.reason(Reason::OtherCredentials);
            return proceed(false);
        }
        Credentials::Witness(m) => m,
    };
    let Some(key) = signer_key(oracle, manifest) else {
        notes.reason(Reason::UnverifiedCredentials);
        return proceed(false);
    };
    if key.is_dev() {
        notes.flag(VerdictFlags::DEV_ATTESTATION);
    }
    let claim = &manifest.seal.claim;
    let record_id = RecordId::from_claim_cbor(&manifest.witness_claim_cbor);
    let record = oracle
        .get_record(&record_id)
        .filter(|r| r.record_id == record_id && r.claim == *claim);
    if !report.binding_intact() {
        notes.reason(Reason::SealBroken);
        let reference = match &record {
            Some(r) => Reference::certain(r, Evidence::Seal),
            None => Reference::unanchored(record_id, claim.clone()),
        };
        let mut v = notes.verdict(Outcome::Altered, Some(reference));
        v.declared.clear();
        return Path::Decided(Box::new(v));
    }
    let Some(record) = record else {
        if !key.history.current().is_trusted() {
            notes.reason(Reason::KeyNotTrusted);
            return proceed(false);
        }
        notes.reason(Reason::SealVerified);
        let reference = Reference::unanchored(record_id, claim.clone());
        return Path::Decided(Box::new(
            notes.verdict(Outcome::SealedUnanchored, Some(reference)),
        ));
    };
    notes.flag(record.flags);
    if key.history.revoked_after(record.anchored_ns) {
        notes.flag(VerdictFlags::KEY_REVOKED_AFTER_ANCHOR);
    }
    if !key.history.active_at(record.anchored_ns) {
        notes.reason(Reason::KeyNotTrustedAtAnchor);
        return proceed(true);
    }
    notes.reason(Reason::SealVerified);
    let outcome = if claim.kind.presentation_only() {
        Outcome::Original
    } else {
        Outcome::MatchesWithChanges
    };
    Path::Decided(Box::new(
        notes.verdict(outcome, Some(Reference::certain(&record, Evidence::Seal))),
    ))
}
