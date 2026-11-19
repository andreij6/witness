use crate::Error;
use crate::jpeg::{self, MAX_SEGMENT_CONTENTS, marker};
use crate::jumbf::{JUMB, JUMD, uuid};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::ops::Range;
use sha2::{Digest, Sha256};

pub const CI: [u8; 2] = *b"JP";
pub const DEFAULT_EN: u16 = 0x0211;
const PACKET_HEADER: usize = 8;
const BOX_HEADER: usize = 8;
pub const MAX_CHUNK: usize = MAX_SEGMENT_CONTENTS - PACKET_HEADER - BOX_HEADER;
pub const MIN_CHUNK: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet<'a> {
    pub en: u16,
    pub z: u32,
    pub data: &'a [u8],
}

pub fn packet(contents: &[u8]) -> Option<Packet<'_>> {
    match contents {
        [b'J', b'P', e0, e1, z0, z1, z2, z3, data @ ..] => Some(Packet {
            en: u16::from_be_bytes([*e0, *e1]),
            z: u32::from_be_bytes([*z0, *z1, *z2, *z3]),
            data,
        }),
        _ => None,
    }
}

fn starts_store(data: &[u8]) -> bool {
    data.len() >= MIN_CHUNK
        && data[4..8] == JUMB
        && data[12..16] == JUMD
        && data[16..32] == uuid::STORE
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout<'a> {
    pub excluded: Vec<Range<usize>>,
    pub store: Option<Cow<'a, [u8]>>,
    pub insert_at: usize,
    pub foreign_en: Vec<u16>,
}

impl Layout<'_> {
    pub fn free_en(&self) -> u16 {
        let mut used = self.foreign_en.clone();
        used.sort_unstable();
        core::iter::once(DEFAULT_EN)
            .chain(1..=u16::MAX)
            .find(|en| used.binary_search(en).is_err())
            .unwrap_or(DEFAULT_EN)
    }
}

struct Assembly<'a> {
    en: u16,
    next_z: u32,
    header: &'a [u8],
    total: usize,
    first: &'a [u8],
    rest: Option<Vec<u8>>,
}

impl<'a> Assembly<'a> {
    fn start(p: Packet<'a>, file_len: usize) -> Result<Self, Error> {
        if p.z != 1 {
            return Err(Error::MalformedSegments);
        }
        let total = u32::from_be_bytes([p.data[0], p.data[1], p.data[2], p.data[3]]) as usize;
        if total < BOX_HEADER || total > file_len || p.data.len() > total {
            return Err(Error::MalformedSegments);
        }
        Ok(Self {
            en: p.en,
            next_z: 2,
            header: &p.data[..BOX_HEADER],
            total,
            first: p.data,
            rest: None,
        })
    }

    fn len(&self) -> usize {
        self.rest.as_ref().map_or(self.first.len(), Vec::len)
    }

    fn push(&mut self, p: Packet<'a>) -> Result<(), Error> {
        let chunk = match p.data.split_at_checked(BOX_HEADER) {
            Some((h, c)) if h == self.header => c,
            _ => return Err(Error::MalformedSegments),
        };
        if p.z != self.next_z || self.len() + chunk.len() > self.total {
            return Err(Error::MalformedSegments);
        }
        self.next_z = self.next_z.checked_add(1).ok_or(Error::MalformedSegments)?;
        let total = self.total;
        let first = self.first;
        self.rest
            .get_or_insert_with(|| {
                let mut v = Vec::with_capacity(total);
                v.extend_from_slice(first);
                v
            })
            .extend_from_slice(chunk);
        Ok(())
    }

    fn finish(self) -> Result<Cow<'a, [u8]>, Error> {
        if self.len() != self.total {
            return Err(Error::MalformedSegments);
        }
        Ok(self.rest.map_or(Cow::Borrowed(self.first), Cow::Owned))
    }
}

pub fn locate(buf: &[u8]) -> Result<Layout<'_>, Error> {
    let mut excluded = Vec::new();
    let mut foreign_en = Vec::new();
    let mut asm: Option<Assembly<'_>> = None;
    let mut insert_at = 2;
    let mut leading = true;
    for seg in jpeg::segments(buf)? {
        let seg = seg?;
        leading &= matches!(seg.marker, marker::APP0 | marker::APP1);
        if leading {
            insert_at = seg.end();
        }
        if seg.marker != marker::APP11 {
            continue;
        }
        let Some(p) = packet(seg.contents) else {
            continue;
        };
        if starts_store(p.data) {
            if asm.is_some() {
                return Err(Error::MultipleStores);
            }
            asm = Some(Assembly::start(p, buf.len())?);
        } else {
            match asm.as_mut() {
                Some(a) if a.en == p.en => a.push(p)?,
                _ => {
                    foreign_en.push(p.en);
                    continue;
                }
            }
        }
        excluded.push(seg.range());
    }
    Ok(Layout {
        excluded,
        store: asm.map(Assembly::finish).transpose()?,
        insert_at,
        foreign_en,
    })
}

pub fn hash_excluding(buf: &[u8], excluded: &[Range<usize>]) -> [u8; 32] {
    let mut h = Sha256::new();
    let mut at = 0;
    for r in excluded {
        h.update(&buf[at..r.start]);
        at = r.end;
    }
    h.update(&buf[at..]);
    h.finalize().into()
}

pub fn data_hash(jpeg: &[u8]) -> Result<[u8; 32], Error> {
    locate(jpeg).map(|l| hash_excluding(jpeg, &l.excluded))
}

pub fn packetized_len(store_len: usize, chunk: usize) -> usize {
    let n = store_len.div_ceil(chunk).max(1);
    store_len + n * (4 + PACKET_HEADER) + (n - 1) * BOX_HEADER
}

pub fn write_packets(out: &mut Vec<u8>, store: &[u8], en: u16, chunk: usize) -> Result<(), Error> {
    if store.len() < MIN_CHUNK || !(MIN_CHUNK..=MAX_CHUNK).contains(&chunk) {
        return Err(Error::TooLarge);
    }
    let header = &store[..BOX_HEADER];
    out.reserve(packetized_len(store.len(), chunk));
    for (i, part) in store.chunks(chunk).enumerate() {
        let z = u32::try_from(i + 1).map_err(|_| Error::TooLarge)?;
        let mut ez = [0u8; PACKET_HEADER];
        ez[..2].copy_from_slice(&CI);
        ez[2..4].copy_from_slice(&en.to_be_bytes());
        ez[4..].copy_from_slice(&z.to_be_bytes());
        let repeat: &[u8] = if i == 0 { &[] } else { header };
        jpeg::write_segment(out, marker::APP11, &[&ez, repeat, part])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
