use crate::Error;
use core::ops::Range;

pub mod marker {
    pub const SOI: u8 = 0xd8;
    pub const EOI: u8 = 0xd9;
    pub const SOS: u8 = 0xda;
    pub const TEM: u8 = 0x01;
    pub const APP0: u8 = 0xe0;
    pub const APP1: u8 = 0xe1;
    pub const APP11: u8 = 0xeb;
    pub const COM: u8 = 0xfe;
    pub const DQT: u8 = 0xdb;
}

const PREFIX: u8 = 0xff;
const SEGMENT_HEADER: usize = 4;
pub const MAX_SEGMENT_CONTENTS: usize = u16::MAX as usize - 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    pub marker: u8,
    pub offset: usize,
    pub contents: &'a [u8],
}

impl Segment<'_> {
    pub const fn end(&self) -> usize {
        self.offset + SEGMENT_HEADER + self.contents.len()
    }

    pub const fn range(&self) -> Range<usize> {
        self.offset..self.end()
    }
}

#[derive(Debug, Clone)]
pub struct Segments<'a> {
    buf: &'a [u8],
    pos: usize,
    in_scan: bool,
}

pub fn segments(buf: &[u8]) -> Result<Segments<'_>, Error> {
    match buf {
        [PREFIX, marker::SOI, ..] => Ok(Segments {
            buf,
            pos: 2,
            in_scan: false,
        }),
        _ => Err(Error::NotJpeg),
    }
}

const fn standalone(m: u8) -> bool {
    matches!(m, marker::TEM | 0xd0..=0xd7)
}

impl<'a> Segments<'a> {
    fn skip_entropy(&mut self) {
        let buf = self.buf;
        let mut p = self.pos;
        while let Some(i) = buf[p..].iter().position(|&b| b == PREFIX) {
            let at = p + i;
            match buf.get(at + 1) {
                Some(0x00 | 0xd0..=0xd7) => p = at + 2,
                Some(_) => {
                    self.pos = at;
                    return;
                }
                None => break,
            }
        }
        self.pos = buf.len();
    }

    fn step(&mut self) -> Result<Option<Segment<'a>>, Error> {
        let buf = self.buf;
        loop {
            if self.in_scan {
                self.in_scan = false;
                self.skip_entropy();
            }
            if self.pos >= buf.len() {
                return Ok(None);
            }
            if buf[self.pos] != PREFIX {
                return Err(Error::MalformedJpeg);
            }
            let fill = buf[self.pos + 1..]
                .iter()
                .position(|&b| b != PREFIX)
                .ok_or(Error::MalformedJpeg)?;
            let at = self.pos + fill;
            let m = buf[at + 1];
            match m {
                marker::EOI => {
                    self.pos = buf.len();
                    return Ok(None);
                }
                0x00 | marker::SOI => return Err(Error::MalformedJpeg),
                m if standalone(m) => self.pos = at + 2,
                m => {
                    let len = buf
                        .get(at + 2..at + 4)
                        .map(|l| usize::from(u16::from_be_bytes([l[0], l[1]])))
                        .ok_or(Error::MalformedJpeg)?;
                    let end = at + 2 + len;
                    if len < 2 || end > buf.len() {
                        return Err(Error::MalformedJpeg);
                    }
                    self.pos = end;
                    self.in_scan = m == marker::SOS;
                    return Ok(Some(Segment {
                        marker: m,
                        offset: at,
                        contents: &buf[at + SEGMENT_HEADER..end],
                    }));
                }
            }
        }
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = Result<Segment<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.step() {
            Ok(s) => s.map(Ok),
            Err(e) => {
                self.pos = self.buf.len();
                self.in_scan = false;
                Some(Err(e))
            }
        }
    }
}

pub fn write_segment(out: &mut alloc::vec::Vec<u8>, m: u8, parts: &[&[u8]]) -> Result<(), Error> {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    if len > MAX_SEGMENT_CONTENTS {
        return Err(Error::TooLarge);
    }
    out.extend_from_slice(&[PREFIX, m]);
    out.extend_from_slice(&((len + 2) as u16).to_be_bytes());
    parts.iter().for_each(|p| out.extend_from_slice(p));
    Ok(())
}

#[cfg(test)]
mod tests;
