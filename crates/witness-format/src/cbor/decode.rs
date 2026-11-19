use super::{Error, Value};
use alloc::vec::Vec;

pub const MAX_DEPTH: u32 = 32;

pub const MAJOR_UINT: u8 = 0;
pub const MAJOR_NINT: u8 = 1;
pub const MAJOR_BYTES: u8 = 2;
pub const MAJOR_TEXT: u8 = 3;
pub const MAJOR_ARRAY: u8 = 4;
pub const MAJOR_MAP: u8 = 5;
pub const MAJOR_SIMPLE: u8 = 7;

const SIMPLE_FALSE: u64 = 20;
const SIMPLE_TRUE: u64 = 21;
const SIMPLE_NULL: u64 = 22;

#[derive(Debug, Clone)]
pub struct Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub const fn position(&self) -> usize {
        self.pos
    }

    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub const fn is_empty(&self) -> bool {
        self.pos == self.buf.len()
    }

    pub const fn finish(&self) -> Result<(), Error> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(Error::Trailing)
        }
    }

    fn take(&mut self, n: u64) -> Result<&'a [u8], Error> {
        if n > self.remaining() as u64 {
            return Err(Error::Eof);
        }
        let start = self.pos;
        self.pos += n as usize;
        Ok(&self.buf[start..self.pos])
    }

    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.buf.get(self.pos).ok_or(Error::Eof)?;
        self.pos += 1;
        Ok(b)
    }

    fn be(&mut self, n: u64) -> Result<u64, Error> {
        Ok(self
            .take(n)?
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
    }

    fn head(&mut self) -> Result<(u8, u64), Error> {
        let ib = self.byte()?;
        let major = ib >> 5;
        let ai = ib & 0x1f;
        if major == 6 {
            return Err(Error::Tag);
        }
        if major == MAJOR_SIMPLE {
            return match ai {
                20..=22 => Ok((major, u64::from(ai))),
                25..=27 => Err(Error::Float),
                28..=30 => Err(Error::Reserved),
                31 => Err(Error::Indefinite),
                _ => Err(Error::Simple),
            };
        }
        let (arg, min) = match ai {
            0..=23 => return Ok((major, u64::from(ai))),
            24 => (self.be(1)?, 24),
            25 => (self.be(2)?, 0x100),
            26 => (self.be(4)?, 0x1_0000),
            27 => (self.be(8)?, 0x1_0000_0000),
            28..=30 => return Err(Error::Reserved),
            _ => return Err(Error::Indefinite),
        };
        if arg < min {
            return Err(Error::NonMinimal);
        }
        Ok((major, arg))
    }

    fn expect(&mut self, major: u8) -> Result<u64, Error> {
        let (m, arg) = self.head()?;
        if m == major {
            Ok(arg)
        } else {
            Err(Error::Type)
        }
    }

    pub fn peek_major(&self) -> Result<u8, Error> {
        self.buf.get(self.pos).map(|b| b >> 5).ok_or(Error::Eof)
    }

    pub fn u64(&mut self) -> Result<u64, Error> {
        self.expect(MAJOR_UINT)
    }

    pub fn i64(&mut self) -> Result<i64, Error> {
        match self.head()? {
            (MAJOR_UINT, v) => i64::try_from(v).map_err(|_| Error::Range),
            (MAJOR_NINT, n) => i64::try_from(n).map(|n| -1 - n).map_err(|_| Error::Range),
            _ => Err(Error::Type),
        }
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], Error> {
        let n = self.expect(MAJOR_BYTES)?;
        self.take(n)
    }

    pub fn byte_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.bytes()?.try_into().map_err(|_| Error::Length)
    }

    pub fn text(&mut self) -> Result<&'a str, Error> {
        let n = self.expect(MAJOR_TEXT)?;
        core::str::from_utf8(self.take(n)?).map_err(|_| Error::Utf8)
    }

    pub fn bool(&mut self) -> Result<bool, Error> {
        match self.head()? {
            (MAJOR_SIMPLE, SIMPLE_FALSE) => Ok(false),
            (MAJOR_SIMPLE, SIMPLE_TRUE) => Ok(true),
            _ => Err(Error::Type),
        }
    }

    pub fn null(&mut self) -> Result<(), Error> {
        match self.head()? {
            (MAJOR_SIMPLE, SIMPLE_NULL) => Ok(()),
            _ => Err(Error::Type),
        }
    }

    pub fn array(&mut self) -> Result<usize, Error> {
        let n = self.expect(MAJOR_ARRAY)?;
        if n > self.remaining() as u64 {
            return Err(Error::Eof);
        }
        Ok(n as usize)
    }

    pub fn array_of(&mut self, len: usize) -> Result<(), Error> {
        if self.array()? == len {
            Ok(())
        } else {
            Err(Error::Length)
        }
    }

    pub fn map(&mut self) -> Result<usize, Error> {
        let n = self.expect(MAJOR_MAP)?;
        if n > (self.remaining() / 2) as u64 {
            return Err(Error::Eof);
        }
        Ok(n as usize)
    }

    pub fn fields(&mut self) -> Result<Fields, Error> {
        Ok(Fields {
            left: self.map()?,
            pending: None,
            last: None,
        })
    }

    pub fn skip(&mut self) -> Result<(), Error> {
        self.skip_at(0)
    }

    pub fn raw_item(&mut self) -> Result<&'a [u8], Error> {
        let start = self.pos;
        self.skip()?;
        Ok(&self.buf[start..self.pos])
    }

    fn skip_at(&mut self, depth: u32) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::Depth);
        }
        let (major, arg) = self.head()?;
        match major {
            MAJOR_BYTES => self.take(arg).map(drop),
            MAJOR_TEXT => core::str::from_utf8(self.take(arg)?)
                .map(drop)
                .map_err(|_| Error::Utf8),
            MAJOR_ARRAY => (0..arg).try_for_each(|_| self.skip_at(depth + 1)),
            MAJOR_MAP => {
                let mut prev: &[u8] = &[];
                for _ in 0..arg {
                    let start = self.pos;
                    self.skip_at(depth + 1)?;
                    let key = &self.buf[start..self.pos];
                    if key <= prev {
                        return Err(Error::KeyOrder);
                    }
                    prev = key;
                    self.skip_at(depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn value(&mut self) -> Result<Value<'a>, Error> {
        self.value_at(0)
    }

    fn value_at(&mut self, depth: u32) -> Result<Value<'a>, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::Depth);
        }
        let (major, arg) = self.head()?;
        Ok(match major {
            MAJOR_UINT => Value::Uint(arg),
            MAJOR_NINT => Value::Nint(arg),
            MAJOR_BYTES => Value::Bytes(self.take(arg)?),
            MAJOR_TEXT => {
                Value::Text(core::str::from_utf8(self.take(arg)?).map_err(|_| Error::Utf8)?)
            }
            MAJOR_ARRAY => {
                if arg > self.remaining() as u64 {
                    return Err(Error::Eof);
                }
                let mut items = Vec::with_capacity(arg as usize);
                for _ in 0..arg {
                    items.push(self.value_at(depth + 1)?);
                }
                Value::Array(items)
            }
            MAJOR_MAP => {
                if arg > (self.remaining() / 2) as u64 {
                    return Err(Error::Eof);
                }
                let mut entries = Vec::with_capacity(arg as usize);
                let mut prev: &[u8] = &[];
                for _ in 0..arg {
                    let start = self.pos;
                    let k = self.value_at(depth + 1)?;
                    let key = &self.buf[start..self.pos];
                    if key <= prev {
                        return Err(Error::KeyOrder);
                    }
                    prev = key;
                    entries.push((k, self.value_at(depth + 1)?));
                }
                Value::Map(entries)
            }
            _ => match arg {
                SIMPLE_FALSE => Value::Bool(false),
                SIMPLE_TRUE => Value::Bool(true),
                _ => Value::Null,
            },
        })
    }
}

#[derive(Debug)]
pub struct Fields {
    left: usize,
    pending: Option<u64>,
    last: Option<u64>,
}

impl Fields {
    fn peek(&mut self, d: &mut Decoder<'_>) -> Result<Option<u64>, Error> {
        if self.pending.is_none() && self.left > 0 {
            let k = d.u64()?;
            if self.last.is_some_and(|l| k <= l) {
                return Err(Error::KeyOrder);
            }
            self.last = Some(k);
            self.pending = Some(k);
        }
        Ok(self.pending)
    }

    pub fn key(&mut self, d: &mut Decoder<'_>, key: u64) -> Result<bool, Error> {
        match self.peek(d)? {
            Some(k) if k == key => {
                self.pending = None;
                self.left -= 1;
                Ok(true)
            }
            Some(k) if k < key => Err(Error::UnknownKey(k)),
            _ => Ok(false),
        }
    }

    pub fn opt<'a, T: super::Decode<'a>>(
        &mut self,
        d: &mut Decoder<'a>,
        key: u64,
    ) -> Result<Option<T>, Error> {
        if self.key(d, key)? {
            T::decode(d).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn req<'a, T: super::Decode<'a>>(
        &mut self,
        d: &mut Decoder<'a>,
        key: u64,
    ) -> Result<T, Error> {
        self.opt(d, key)?.ok_or(Error::MissingKey(key))
    }

    pub fn version(&mut self, d: &mut Decoder<'_>, expected: u64) -> Result<(), Error> {
        match self.req::<u64>(d, 0)? {
            v if v == expected => Ok(()),
            v => Err(Error::Version(v)),
        }
    }

    pub fn finish(mut self, d: &mut Decoder<'_>) -> Result<(), Error> {
        match self.peek(d)? {
            Some(k) => Err(Error::UnknownKey(k)),
            None => Ok(()),
        }
    }
}
