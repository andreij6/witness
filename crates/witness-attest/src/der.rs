use crate::Error;

pub const CLASS_UNIVERSAL: u8 = 0;
pub const CLASS_CONTEXT: u8 = 2;

const MAX_TAG_NUMBER: u32 = 1 << 21;
const MAX_LENGTH_OCTETS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub class: u8,
    pub constructed: bool,
    pub number: u32,
}

impl Tag {
    pub const BOOLEAN: Self = Self::universal(1, false);
    pub const INTEGER: Self = Self::universal(2, false);
    pub const BIT_STRING: Self = Self::universal(3, false);
    pub const OCTET_STRING: Self = Self::universal(4, false);
    pub const NULL: Self = Self::universal(5, false);
    pub const OID: Self = Self::universal(6, false);
    pub const ENUMERATED: Self = Self::universal(10, false);
    pub const SEQUENCE: Self = Self::universal(16, true);
    pub const SET: Self = Self::universal(17, true);
    pub const UTC_TIME: Self = Self::universal(23, false);
    pub const GENERALIZED_TIME: Self = Self::universal(24, false);

    pub const fn universal(number: u32, constructed: bool) -> Self {
        Self {
            class: CLASS_UNIVERSAL,
            constructed,
            number,
        }
    }

    pub const fn context(number: u32, constructed: bool) -> Self {
        Self {
            class: CLASS_CONTEXT,
            constructed,
            number,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tlv<'a> {
    pub tag: Tag,
    pub value: &'a [u8],
    pub raw: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    buf: &'a [u8],
}

fn read_tag(buf: &[u8]) -> Result<(Tag, usize), Error> {
    let &first = buf.first().ok_or(Error::Der)?;
    let class = first >> 6;
    let constructed = first & 0x20 != 0;
    let low = u32::from(first & 0x1f);
    if low != 0x1f {
        return Ok((
            Tag {
                class,
                constructed,
                number: low,
            },
            1,
        ));
    }
    let mut number = 0u32;
    for (i, &b) in buf.iter().enumerate().skip(1) {
        if i == 1 && b == 0x80 {
            return Err(Error::Der);
        }
        number = (number << 7) | u32::from(b & 0x7f);
        if number >= MAX_TAG_NUMBER {
            return Err(Error::Der);
        }
        if b & 0x80 == 0 {
            if number < 0x1f {
                return Err(Error::Der);
            }
            return Ok((
                Tag {
                    class,
                    constructed,
                    number,
                },
                i + 1,
            ));
        }
    }
    Err(Error::Der)
}

fn read_length(buf: &[u8]) -> Result<(usize, usize), Error> {
    let &first = buf.first().ok_or(Error::Der)?;
    if first < 0x80 {
        return Ok((usize::from(first), 1));
    }
    let n = usize::from(first & 0x7f);
    if n == 0 || n > MAX_LENGTH_OCTETS || buf.len() <= n || buf[1] == 0 {
        return Err(Error::Der);
    }
    let len = buf[1..=n]
        .iter()
        .fold(0usize, |acc, &b| (acc << 8) | usize::from(b));
    if len < 0x80 {
        return Err(Error::Der);
    }
    Ok((len, n + 1))
}

impl<'a> Reader<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    pub const fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn peek_tag(&self) -> Option<Tag> {
        read_tag(self.buf).ok().map(|(t, _)| t)
    }

    pub fn read(&mut self) -> Result<Tlv<'a>, Error> {
        let (tag, tag_len) = read_tag(self.buf)?;
        let (len, len_len) = read_length(&self.buf[tag_len..])?;
        let header = tag_len + len_len;
        let end = header.checked_add(len).ok_or(Error::Der)?;
        if end > self.buf.len() {
            return Err(Error::Der);
        }
        let (raw, rest) = self.buf.split_at(end);
        self.buf = rest;
        Ok(Tlv {
            tag,
            value: &raw[header..],
            raw,
        })
    }

    pub fn expect(&mut self, tag: Tag) -> Result<&'a [u8], Error> {
        self.expect_tlv(tag).map(|t| t.value)
    }

    pub fn expect_tlv(&mut self, tag: Tag) -> Result<Tlv<'a>, Error> {
        let tlv = self.read()?;
        if tlv.tag == tag {
            Ok(tlv)
        } else {
            Err(Error::Der)
        }
    }

    pub fn optional(&mut self, tag: Tag) -> Result<Option<&'a [u8]>, Error> {
        if self.peek_tag() == Some(tag) {
            self.expect(tag).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn finish(self) -> Result<(), Error> {
        if self.buf.is_empty() {
            Ok(())
        } else {
            Err(Error::Der)
        }
    }
}

pub fn single(buf: &[u8], tag: Tag) -> Result<&[u8], Error> {
    let mut r = Reader::new(buf);
    let value = r.expect(tag)?;
    r.finish()?;
    Ok(value)
}

pub fn integer_bytes(value: &[u8]) -> Result<&[u8], Error> {
    match value {
        [] => Err(Error::Der),
        [0x00, next, ..] if next & 0x80 == 0 => Err(Error::Der),
        [0xff, next, ..] if next & 0x80 != 0 => Err(Error::Der),
        _ => Ok(value),
    }
}

pub fn uint(value: &[u8]) -> Result<u64, Error> {
    let bytes = integer_bytes(value)?;
    if bytes[0] & 0x80 != 0 {
        return Err(Error::Der);
    }
    let magnitude = if bytes[0] == 0 { &bytes[1..] } else { bytes };
    if magnitude.len() > 8 {
        return Err(Error::Der);
    }
    Ok(magnitude
        .iter()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
}

pub fn unsigned_magnitude(value: &[u8]) -> Result<&[u8], Error> {
    let bytes = integer_bytes(value)?;
    if bytes[0] & 0x80 != 0 {
        return Err(Error::Der);
    }
    Ok(if bytes[0] == 0 { &bytes[1..] } else { bytes })
}

pub fn boolean(value: &[u8]) -> Result<bool, Error> {
    match value {
        [0x00] => Ok(false),
        [0xff] => Ok(true),
        _ => Err(Error::Der),
    }
}

pub fn bit_string(value: &[u8]) -> Result<(u8, &[u8]), Error> {
    let (&unused, bits) = value.split_first().ok_or(Error::Der)?;
    if unused > 7 || (bits.is_empty() && unused != 0) {
        return Err(Error::Der);
    }
    if let Some(&last) = bits.last()
        && last & ((1u8 << unused) - 1) != 0
    {
        return Err(Error::Der);
    }
    Ok((unused, bits))
}

pub fn octet_aligned_bits(value: &[u8]) -> Result<&[u8], Error> {
    match bit_string(value)? {
        (0, bits) => Ok(bits),
        _ => Err(Error::Der),
    }
}

#[cfg(test)]
mod tests;
