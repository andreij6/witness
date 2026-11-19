mod decode;
mod encode;
mod error;
mod value;

#[cfg(test)]
mod tests;

pub use decode::{Decoder, Fields, MAX_DEPTH};
pub use encode::Encoder;
pub use error::Error;
pub use value::Value;

use alloc::string::String;
use alloc::vec::Vec;

pub trait Encode {
    fn encode(&self, e: &mut Encoder<'_>);
}

pub trait Decode<'a>: Sized {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error>;
}

pub fn encode_into<T: Encode + ?Sized>(v: &T, out: &mut Vec<u8>) {
    v.encode(&mut Encoder::new(out));
}

pub fn to_vec<T: Encode + ?Sized>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(v, &mut out);
    out
}

pub fn from_slice<'a, T: Decode<'a>>(bytes: &'a [u8]) -> Result<T, Error> {
    let mut d = Decoder::new(bytes);
    let v = T::decode(&mut d)?;
    d.finish()?;
    Ok(v)
}

pub fn from_slice_canonical<'a, T: Decode<'a> + Encode>(bytes: &'a [u8]) -> Result<T, Error> {
    let v = from_slice::<T>(bytes)?;
    let mut out = Vec::with_capacity(bytes.len());
    encode_into(&v, &mut out);
    if out == bytes {
        Ok(v)
    } else {
        Err(Error::NotCanonical)
    }
}

pub fn validate(bytes: &[u8]) -> Result<(), Error> {
    let mut d = Decoder::new(bytes);
    d.skip()?;
    d.finish()
}

macro_rules! uint_codec {
    ($($t:ty),+) => {$(
        impl Encode for $t {
            fn encode(&self, e: &mut Encoder<'_>) {
                e.u64(u64::from(*self));
            }
        }
        impl<'a> Decode<'a> for $t {
            fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
                <$t>::try_from(d.u64()?).map_err(|_| Error::Range)
            }
        }
    )+};
}

macro_rules! int_codec {
    ($($t:ty),+) => {$(
        impl Encode for $t {
            fn encode(&self, e: &mut Encoder<'_>) {
                e.i64(i64::from(*self));
            }
        }
        impl<'a> Decode<'a> for $t {
            fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
                <$t>::try_from(d.i64()?).map_err(|_| Error::Range)
            }
        }
    )+};
}

uint_codec!(u8, u16, u32, u64);
int_codec!(i8, i16, i32, i64);

impl Encode for bool {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.bool(*self);
    }
}

impl<'a> Decode<'a> for bool {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.bool()
    }
}

impl<const N: usize> Encode for [u8; N] {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.bytes(self);
    }
}

impl<'a, const N: usize> Decode<'a> for [u8; N] {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.byte_array()
    }
}

impl Encode for [u8] {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.bytes(self);
    }
}

impl<'a> Decode<'a> for &'a [u8] {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.bytes()
    }
}

impl Encode for str {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.text(self);
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.text()
    }
}

impl Encode for String {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.text(self);
    }
}

impl<'a> Decode<'a> for String {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.text().map(String::from)
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.array(self.len());
        self.iter().for_each(|v| v.encode(e));
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let n = d.array()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(T::decode(d)?);
        }
        Ok(out)
    }
}

impl<A: Encode, B: Encode> Encode for (A, B) {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.array(2);
        self.0.encode(e);
        self.1.encode(e);
    }
}

impl<'a, A: Decode<'a>, B: Decode<'a>> Decode<'a> for (A, B) {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.array_of(2)?;
        Ok((A::decode(d)?, B::decode(d)?))
    }
}

impl<T: Encode + ?Sized> Encode for &T {
    fn encode(&self, e: &mut Encoder<'_>) {
        (**self).encode(e);
    }
}
