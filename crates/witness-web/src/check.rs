use crate::error::{Error, Result};
use crate::oracle::{Answers, Need, ReplayOracle};
use crate::render;
use crate::wire::CheckOptions;
use serde_json::{Value, json};
use witness_core::c2pa::{Credentials, TrustAnchor, verify_jpeg};
use witness_core::format::RecordId;
use witness_core::image::{Format, sniff};
use witness_core::verdict::{Options, Verdict, judge};

pub const UNPINNED_ANCHOR_DER: &[u8] =
    include_bytes!("../../../canisters/common/unpinned_anchor.der");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorTrust {
    Pinned,
    Dev,
    Missing,
}

impl AnchorTrust {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Dev => "dev",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub verdict: Verdict,
    pub need: Need,
    pub anchor: AnchorTrust,
}

impl Step {
    pub fn done(&self) -> bool {
        self.need.is_empty()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "anchor": self.anchor.name(),
            "done": self.done(),
            "need": self.need,
            "verdict": render::verdict(&self.verdict),
        })
    }
}

impl CheckOptions {
    pub fn parse(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn trust(&self) -> AnchorTrust {
        match (self.anchor.0.is_empty(), self.dev_anchor) {
            (true, _) => AnchorTrust::Missing,
            (false, true) => AnchorTrust::Dev,
            (false, false) => AnchorTrust::Pinned,
        }
    }

    pub fn anchor(&self) -> Result<TrustAnchor> {
        let der = match self.trust() {
            AnchorTrust::Missing => UNPINNED_ANCHOR_DER,
            _ => &self.anchor.0,
        };
        TrustAnchor::from_der(der).map_err(|_| Error::Malformed("anchor"))
    }

    pub fn options<'a>(&self, anchor: &'a TrustAnchor) -> Options<'a> {
        let mut o = match self.top_k {
            Some(k) => Options::new(anchor, k),
            None => Options::web(anchor),
        };
        if let Some(at) = self.at_s {
            o = o.at(at);
        }
        if let Some(max) = self.max_pixels {
            o.max_pixels = max;
        }
        o
    }
}

pub fn step(input: &[u8], options: &CheckOptions, answers: &Answers) -> Result<Step> {
    let anchor = options.anchor()?;
    let oracle = ReplayOracle::new(answers);
    let verdict = judge(input, &oracle, &options.options(&anchor));
    Ok(Step {
        verdict,
        need: oracle.into_need(),
        anchor: options.trust(),
    })
}

pub fn credentials(input: &[u8], options: &CheckOptions) -> Result<Value> {
    if sniff(input) != Some(Format::Jpeg) {
        return Ok(json!({"kind": "none"}));
    }
    let anchor = options.anchor()?;
    let Ok(report) = verify_jpeg(input, &anchor, options.at_s) else {
        let kind = match options.trust() {
            AnchorTrust::Missing => "unverified",
            _ => "invalid",
        };
        return Ok(json!({"kind": kind}));
    };
    Ok(match &report.credentials {
        Credentials::Absent => {
            json!({"kind": "absent", "assetHash": hex::encode(report.data_hash)})
        }
        Credentials::Other => json!({"kind": "other", "assetHash": hex::encode(report.data_hash)}),
        Credentials::Witness(m) => json!({
            "kind": "witness",
            "recordId": RecordId::from_claim_cbor(&m.witness_claim_cbor).to_string(),
            "bindingIntact": report.binding_intact(),
            "assetHash": hex::encode(report.data_hash),
            "claim": render::claim(&m.seal.claim),
        }),
    })
}

pub fn plan_json(input: &[u8], options: &str) -> Result<String> {
    let options = CheckOptions::parse(options)?;
    let mut out = step(input, &options, &Answers::default())?.to_json();
    out["credentials"] = credentials(input, &options)?;
    Ok(out.to_string())
}

pub fn judge_json(input: &[u8], options: &str, fetched: &str) -> Result<String> {
    let options = CheckOptions::parse(options)?;
    let answers = Answers::parse(fetched)?;
    Ok(step(input, &options, &answers)?.to_json().to_string())
}

#[cfg(test)]
mod tests;
