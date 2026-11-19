use serde_json::{Value, json};
use witness_core::format::{
    BlurClass, Edit, ImageFormat, Kind, Outcome, VerdictFlags, WitnessClaim,
};
use witness_core::verdict::{Evidence, Reference, Verdict};

pub const FLAG_NAMES: [&str; 7] = [
    "KEY_REVOKED_AFTER_ANCHOR",
    "LATE_ANCHOR",
    "CLOCK_MISMATCH",
    "COULDNT_COMPARE",
    "NO_PUBLIC_REFERENCE",
    "DEV_ATTESTATION",
    "TRUNCATED_CANDIDATES",
];

pub fn outcome_name(o: Outcome) -> &'static str {
    match o {
        Outcome::Original => "ORIGINAL",
        Outcome::MatchesWithChanges => "MATCHES_WITH_CHANGES",
        Outcome::Altered => "ALTERED",
        Outcome::NoRecord => "NO_RECORD",
        Outcome::SealedUnanchored => "SEALED_UNANCHORED",
    }
}

pub fn flag_names(flags: VerdictFlags) -> Vec<&'static str> {
    VerdictFlags::FLAGS
        .iter()
        .zip(FLAG_NAMES)
        .filter(|(f, _)| flags.contains(**f))
        .map(|(_, n)| n)
        .collect()
}

fn evidence_name(e: Evidence) -> &'static str {
    match e {
        Evidence::Seal => "seal",
        Evidence::ExactHash => "exact_hash",
        Evidence::Pixels => "pixels",
        Evidence::Grid => "grid",
    }
}

fn format_name(f: ImageFormat) -> &'static str {
    match f {
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::Png => "png",
        ImageFormat::Webp => "webp",
        ImageFormat::Avif => "avif",
        ImageFormat::Heic => "heic",
    }
}

fn blur_name(c: BlurClass) -> &'static str {
    match c {
        BlurClass::Face => "face",
        BlurClass::Plate => "plate",
        BlurClass::HouseNo => "house_no",
        BlurClass::Other => "other",
    }
}

pub fn edit(e: &Edit) -> Value {
    canonical(match *e {
        Edit::Resize { w, h, filter } => {
            json!({"kind": "resize", "w": w, "h": h, "filter": filter})
        }
        Edit::Recompress { format, quality } => {
            json!({"kind": "recompress", "format": format_name(format), "quality": quality})
        }
        Edit::Orient { orientation } => json!({"kind": "orient", "orientation": orientation}),
        Edit::Crop(r) => json!({"kind": "crop", "x": r.x, "y": r.y, "w": r.w, "h": r.h}),
        Edit::Rotate { deg_e2 } => json!({"kind": "rotate", "degE2": deg_e2}),
        Edit::Tone {
            exposure_e2,
            contrast_e2,
            saturation_e2,
        } => json!({
            "kind": "tone",
            "exposureE2": exposure_e2,
            "contrastE2": contrast_e2,
            "saturationE2": saturation_e2,
        }),
        Edit::Filter { id, strength_e2 } => {
            json!({"kind": "filter", "id": id, "strengthE2": strength_e2})
        }
        Edit::Blur {
            rect,
            class,
            sigma_e2,
        } => json!({
            "kind": "blur",
            "x": rect.x,
            "y": rect.y,
            "w": rect.w,
            "h": rect.h,
            "class": blur_name(class),
            "sigmaE2": sigma_e2,
        }),
    })
}

pub fn claim(c: &WitnessClaim) -> Value {
    let kind = match &c.kind {
        Kind::Original => json!({"type": "original"}),
        Kind::Derivative {
            parent,
            root,
            edits,
        } => json!({
            "type": "derivative",
            "parent": parent.to_string(),
            "root": root.to_u64().to_string(),
            "edits": edits.iter().map(edit).collect::<Vec<_>>(),
        }),
    };
    canonical(json!({
        "captureId": c.capture_id.to_u64().to_string(),
        "kind": kind,
        "assetHash": hex::encode(c.asset_hash),
        "pixelHash": hex::encode(c.pixel_hash),
        "width": c.dims.width,
        "height": c.dims.height,
        "wallMs": c.time.wall_ms.to_string(),
        "keyId": hex::encode(c.key.key_id.as_bytes()),
        "counter": c.key.counter.to_string(),
        "coarse": c.coarse.as_ref().map(|g| g.as_str().to_owned()),
    }))
}

pub fn claim_detail(c: &WitnessClaim) -> Value {
    let mut v = claim(c);
    v["fpCommit"] = json!(hex::encode(c.fp_commit));
    v["gnssMs"] = json!(c.time.gnss_ms.map(|g| g.to_string()));
    v["elapsedNs"] = json!(c.time.elapsed_ns.to_string());
    v["bootCount"] = json!(c.time.boot_count);
    v["location"] = json!({
        "commit": c.loc.commit.map(hex::encode),
        "confidence": c.loc.confidence.code(),
        "reasons": c.loc.reasons.bits(),
    });
    v["checks"] = json!({
        "keySecurity": c.checks.key_security.code(),
        "boot": c.checks.boot.code(),
        "deviceLocked": c.checks.device_locked,
        "mockLocation": c.checks.mock_location,
        "gnss": c.checks.gnss.bits(),
        "recapture": c.checks.recapture,
    });
    v["certSerial"] = json!(hex::encode(c.cert_serial.as_bytes()));
    canonical(v)
}

fn reference(r: &Reference) -> Value {
    json!({
        "recordId": r.record_id.to_string(),
        "claim": claim(&r.claim),
        "anchor": r.anchor.map(|a| json!({
            "recordIx": a.record_ix.to_string(),
            "anchoredNs": a.anchored_ns.to_string(),
        })),
        "evidence": evidence_name(r.evidence),
        "confidenceQ16": r.confidence_q16,
        "comparedQ16": r.compared_q16,
        "changedQ16": r.changed_q16,
    })
}

pub fn verdict(v: &Verdict) -> Value {
    canonical(json!({
        "version": v.version,
        "outcome": outcome_name(v.outcome),
        "outcomeCode": v.outcome.code(),
        "flags": v.flags.bits(),
        "flagNames": flag_names(v.flags),
        "reasons": v.reasons.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>(),
        "reasonBits": v.reasons.bits(),
        "declared": v.declared.iter().map(edit).collect::<Vec<_>>(),
        "regions": v.regions.iter().map(|r| json!({"x": r.x, "y": r.y, "w": r.w, "h": r.h})).collect::<Vec<_>>(),
        "recordId": v.reference.as_ref().map(|r| r.record_id.to_string()),
        "reference": v.reference.as_ref().map(reference),
    }))
}

pub fn claim_cbor_json(bytes: &[u8]) -> crate::Result<String> {
    let c = WitnessClaim::from_cbor(bytes).map_err(|_| crate::error::Error::Malformed("claim"))?;
    Ok(claim_detail(&c).to_string())
}

fn canonical(mut v: Value) -> Value {
    v.sort_all_objects();
    v
}

#[cfg(test)]
mod tests;
