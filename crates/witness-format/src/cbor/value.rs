use super::{Decode, Decoder, Encoder, Error};
use alloc::vec::Vec;
use core::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    Uint(u64),
    Nint(u64),
    Bytes(&'a [u8]),
    Text(&'a str),
    Array(Vec<Value<'a>>),
    Map(Vec<(Value<'a>, Value<'a>)>),
    Bool(bool),
    Null,
}

impl Value<'_> {
    pub fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        let mut e = Encoder::new(out);
        self.write(&mut e)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        self.encode_into(&mut out)?;
        Ok(out)
    }

    fn write(&self, e: &mut Encoder<'_>) -> Result<(), Error> {
        match self {
            Self::Uint(v) => e.u64(*v),
            Self::Nint(n) => e.nint(*n),
            Self::Bytes(b) => e.bytes(b),
            Self::Text(s) => e.text(s),
            Self::Bool(b) => e.bool(*b),
            Self::Null => e.null(),
            Self::Array(items) => {
                e.array(items.len());
                items.iter().try_for_each(|v| v.write(e))?;
            }
            Self::Map(entries) => write_map(entries, e)?,
        }
        Ok(())
    }
}

fn write_map(entries: &[(Value<'_>, Value<'_>)], e: &mut Encoder<'_>) -> Result<(), Error> {
    let mut keys = Vec::new();
    let mut spans: Vec<(Range<usize>, usize)> = Vec::with_capacity(entries.len());
    for (i, (k, _)) in entries.iter().enumerate() {
        let start = keys.len();
        k.encode_into(&mut keys)?;
        spans.push((start..keys.len(), i));
    }
    spans.sort_unstable_by(|a, b| keys[a.0.clone()].cmp(&keys[b.0.clone()]));
    if spans
        .windows(2)
        .any(|w| keys[w[0].0.clone()] == keys[w[1].0.clone()])
    {
        return Err(Error::KeyOrder);
    }
    e.map(entries.len());
    for (span, i) in spans {
        e.raw(&keys[span]);
        entries[i].1.write(e)?;
    }
    Ok(())
}

impl<'a> Decode<'a> for Value<'a> {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.value()
    }
}
