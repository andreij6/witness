use crate::error::{Error as WebError, Result};
use serde_json::{Value, json};
use witness_core::format::cbor::{self, Decode, Decoder, Encode, Encoder, Error};
use witness_core::format::{RecordId, Submission};

pub const VERSION: u64 = 1;
pub const MAX_ITEMS: usize = 64;
pub const MAX_TITLE: usize = 512;
pub const MAX_SUMMARY: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseItem<'a> {
    pub record_id: RecordId,
    pub jpeg: &'a [u8],
    pub submission: Option<Submission>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBundle<'a> {
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub items: Vec<CaseItem<'a>>,
}

impl Encode for CaseItem<'_> {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(2 + usize::from(self.submission.is_some()));
        e.field(0, &self.record_id);
        e.field(1, self.jpeg);
        e.opt_field(2, self.submission.as_ref());
    }
}

impl<'a> Decode<'a> for CaseItem<'a> {
    fn decode(d: &mut Decoder<'a>) -> core::result::Result<Self, Error> {
        let mut f = d.fields()?;
        let item = Self {
            record_id: f.req(d, 0)?,
            jpeg: f.req(d, 1)?,
            submission: f.opt(d, 2)?,
        };
        f.finish(d)?;
        if item.jpeg.is_empty() {
            return Err(Error::Length);
        }
        if item
            .submission
            .as_ref()
            .is_some_and(|s| s.record_id() != item.record_id)
        {
            return Err(Error::Invalid);
        }
        Ok(item)
    }
}

impl Encode for CaseBundle<'_> {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(3 + usize::from(self.summary.is_some()));
        e.field(0, &VERSION);
        e.field(1, self.title);
        e.opt_field(2, self.summary.as_ref());
        e.field(3, &self.items);
    }
}

impl<'a> Decode<'a> for CaseBundle<'a> {
    fn decode(d: &mut Decoder<'a>) -> core::result::Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, VERSION)?;
        let bundle = Self {
            title: f.req(d, 1)?,
            summary: f.opt(d, 2)?,
            items: f.req(d, 3)?,
        };
        f.finish(d)?;
        let fits = bundle.title.len() <= MAX_TITLE
            && bundle.summary.is_none_or(|s| s.len() <= MAX_SUMMARY)
            && !bundle.items.is_empty()
            && bundle.items.len() <= MAX_ITEMS;
        if fits { Ok(bundle) } else { Err(Error::Length) }
    }
}

impl<'a> CaseBundle<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        cbor::from_slice_canonical(bytes).map_err(|_| WebError::Malformed("case bundle"))
    }

    pub fn to_cbor(&self) -> Vec<u8> {
        cbor::to_vec(self)
    }

    pub fn to_json(&self, base: &[u8]) -> Value {
        let start = base.as_ptr() as usize;
        let items: Vec<Value> = self
            .items
            .iter()
            .map(|i| {
                json!({
                    "recordId": i.record_id.to_string(),
                    "jpeg": {"offset": i.jpeg.as_ptr() as usize - start, "len": i.jpeg.len()},
                    "submission": i.submission.as_ref().map(|s| json!({
                        "claim": hex::encode(&s.claim_cbor),
                        "c2paClaim": hex::encode(&s.c2pa_claim_cbor),
                        "cose": hex::encode(&s.cose),
                    })),
                })
            })
            .collect();
        json!({
            "version": VERSION,
            "title": self.title,
            "summary": self.summary,
            "items": items,
        })
    }
}

pub fn parse_json(bytes: &[u8]) -> Result<String> {
    Ok(CaseBundle::parse(bytes)?.to_json(bytes).to_string())
}

#[cfg(test)]
mod tests;
