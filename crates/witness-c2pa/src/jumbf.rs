use crate::Error;
use alloc::vec::Vec;

pub type Uuid = [u8; 16];
pub type FourCc = [u8; 4];

pub const JUMB: FourCc = *b"jumb";
pub const JUMD: FourCc = *b"jumd";
pub const CBOR: FourCc = *b"cbor";
pub const JSON: FourCc = *b"json";
pub const BFDB: FourCc = *b"bfdb";
pub const BIDB: FourCc = *b"bidb";

const ISO_SUFFIX: [u8; 12] = [
    0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

pub const fn iso_uuid(tag: FourCc) -> Uuid {
    let mut u = [0u8; 16];
    let mut i = 0;
    while i < 4 {
        u[i] = tag[i];
        i += 1;
    }
    while i < 16 {
        u[i] = ISO_SUFFIX[i - 4];
        i += 1;
    }
    u
}

pub mod uuid {
    use super::{Uuid, iso_uuid};

    pub const STORE: Uuid = iso_uuid(*b"c2pa");
    pub const MANIFEST: Uuid = iso_uuid(*b"c2ma");
    pub const UPDATE_MANIFEST: Uuid = iso_uuid(*b"c2um");
    pub const ASSERTIONS: Uuid = iso_uuid(*b"c2as");
    pub const CLAIM: Uuid = iso_uuid(*b"c2cl");
    pub const SIGNATURE: Uuid = iso_uuid(*b"c2cs");
    pub const CBOR: Uuid = iso_uuid(*b"cbor");
    pub const JSON: Uuid = iso_uuid(*b"json");
    pub const EMBEDDED_FILE: Uuid = [
        0x40, 0xcb, 0x0c, 0x32, 0xbb, 0x8a, 0x48, 0x9d, 0xa7, 0x0b, 0x2a, 0xd6, 0xf4, 0x7f, 0x43,
        0x69,
    ];
}

pub mod toggles {
    pub const REQUESTABLE: u8 = 0x01;
    pub const LABEL: u8 = 0x02;
    pub const ID: u8 = 0x04;
    pub const HASH: u8 = 0x08;
    pub const PRIVATE: u8 = 0x10;
}

const HEADER: usize = 8;
const XL_HEADER: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jbox<'a> {
    pub kind: FourCc,
    pub payload: &'a [u8],
}

pub fn split_box(data: &[u8]) -> Result<(Jbox<'_>, &[u8]), Error> {
    let lbox = read_u32(data, 0)? as usize;
    let kind: FourCc = data
        .get(4..8)
        .and_then(|k| k.try_into().ok())
        .ok_or(Error::MalformedJumbf)?;
    let (header, total) = match lbox {
        1 => {
            let xl = data
                .get(8..16)
                .map(|b| u64::from_be_bytes(b.try_into().unwrap_or_default()))
                .ok_or(Error::MalformedJumbf)?;
            (
                XL_HEADER,
                usize::try_from(xl).map_err(|_| Error::MalformedJumbf)?,
            )
        }
        n if n >= HEADER => (HEADER, n),
        _ => return Err(Error::MalformedJumbf),
    };
    if total < header || total > data.len() {
        return Err(Error::MalformedJumbf);
    }
    Ok((
        Jbox {
            kind,
            payload: &data[header..total],
        },
        &data[total..],
    ))
}

pub fn single_box(data: &[u8]) -> Result<Jbox<'_>, Error> {
    match split_box(data)? {
        (b, []) => Ok(b),
        _ => Err(Error::MalformedJumbf),
    }
}

fn read_u32(data: &[u8], at: usize) -> Result<u32, Error> {
    data.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(Error::MalformedJumbf)
}

#[derive(Debug, Clone, Copy)]
pub struct Boxes<'a> {
    rest: &'a [u8],
}

impl<'a> Boxes<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { rest: data }
    }
}

impl<'a> Iterator for Boxes<'a> {
    type Item = Result<Jbox<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        Some(match split_box(self.rest) {
            Ok((b, rest)) => {
                self.rest = rest;
                Ok(b)
            }
            Err(e) => {
                self.rest = &[];
                Err(e)
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Description<'a> {
    pub uuid: Uuid,
    pub toggles: u8,
    pub label: Option<&'a str>,
    pub id: Option<u32>,
    pub hash: Option<&'a [u8; 32]>,
    pub private: Option<Jbox<'a>>,
}

impl<'a> Description<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        let uuid: Uuid = payload
            .get(..16)
            .and_then(|u| u.try_into().ok())
            .ok_or(Error::MalformedJumbf)?;
        let toggles = *payload.get(16).ok_or(Error::MalformedJumbf)?;
        if toggles & !0x1f != 0 {
            return Err(Error::MalformedJumbf);
        }
        let mut rest = &payload[17..];
        let label = if toggles & toggles::LABEL != 0 {
            let end = rest
                .iter()
                .position(|&b| b == 0)
                .ok_or(Error::MalformedJumbf)?;
            let label = core::str::from_utf8(&rest[..end]).map_err(|_| Error::MalformedJumbf)?;
            rest = &rest[end + 1..];
            Some(label)
        } else {
            None
        };
        let id = if toggles & toggles::ID != 0 {
            let id = read_u32(rest, 0)?;
            rest = &rest[4..];
            Some(id)
        } else {
            None
        };
        let hash = if toggles & toggles::HASH != 0 {
            let h: &[u8; 32] = rest
                .get(..32)
                .and_then(|h| h.try_into().ok())
                .ok_or(Error::MalformedJumbf)?;
            rest = &rest[32..];
            Some(h)
        } else {
            None
        };
        let private = if toggles & toggles::PRIVATE != 0 {
            let (b, tail) = split_box(rest)?;
            rest = tail;
            Some(b)
        } else {
            None
        };
        if !rest.is_empty() {
            return Err(Error::MalformedJumbf);
        }
        Ok(Self {
            uuid,
            toggles,
            label,
            id,
            hash,
            private,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuperBox<'a> {
    pub desc: Description<'a>,
    pub payload: &'a [u8],
    pub content: &'a [u8],
}

impl<'a> SuperBox<'a> {
    pub fn parse(b: Jbox<'a>) -> Result<Self, Error> {
        if b.kind != JUMB {
            return Err(Error::MalformedJumbf);
        }
        let (d, content) = split_box(b.payload)?;
        if d.kind != JUMD {
            return Err(Error::MalformedJumbf);
        }
        Ok(Self {
            desc: Description::parse(d.payload)?,
            payload: b.payload,
            content,
        })
    }

    pub fn from_bytes(data: &'a [u8]) -> Result<Self, Error> {
        Self::parse(single_box(data)?)
    }

    pub fn label(&self) -> Option<&'a str> {
        self.desc.label
    }

    pub fn children(&self) -> Boxes<'a> {
        Boxes::new(self.content)
    }

    pub fn superboxes(self) -> impl Iterator<Item = Result<SuperBox<'a>, Error>> + 'a {
        self.children().map(|b| b.and_then(SuperBox::parse))
    }

    pub fn only_child(&self, kind: FourCc) -> Result<&'a [u8], Error> {
        match single_box(self.content)? {
            b if b.kind == kind => Ok(b.payload),
            _ => Err(Error::MalformedJumbf),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedFileDescription<'a> {
    pub media_type: &'a str,
    pub file_name: Option<&'a str>,
    pub external: bool,
}

impl<'a> EmbeddedFileDescription<'a> {
    const FILE_NAME: u8 = 0x01;
    const EXTERNAL: u8 = 0x02;

    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        let (&t, rest) = payload.split_first().ok_or(Error::MalformedJumbf)?;
        if t & !(Self::FILE_NAME | Self::EXTERNAL) != 0 {
            return Err(Error::MalformedJumbf);
        }
        let (media_type, rest) = cstr(rest)?;
        let (file_name, rest) = if t & Self::FILE_NAME != 0 {
            let (n, r) = cstr(rest)?;
            (Some(n), r)
        } else {
            (None, rest)
        };
        if !rest.is_empty() {
            return Err(Error::MalformedJumbf);
        }
        Ok(Self {
            media_type,
            file_name,
            external: t & Self::EXTERNAL != 0,
        })
    }

    pub fn write(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        let at = begin(out, BFDB);
        let t = if self.file_name.is_some() {
            Self::FILE_NAME
        } else {
            0
        } | if self.external { Self::EXTERNAL } else { 0 };
        out.push(t);
        put_cstr(out, self.media_type)?;
        if let Some(n) = self.file_name {
            put_cstr(out, n)?;
        }
        end(out, at)
    }
}

fn cstr(data: &[u8]) -> Result<(&str, &[u8]), Error> {
    let end = data
        .iter()
        .position(|&b| b == 0)
        .ok_or(Error::MalformedJumbf)?;
    let s = core::str::from_utf8(&data[..end]).map_err(|_| Error::MalformedJumbf)?;
    Ok((s, &data[end + 1..]))
}

fn put_cstr(out: &mut Vec<u8>, s: &str) -> Result<(), Error> {
    if s.as_bytes().contains(&0) {
        return Err(Error::MalformedJumbf);
    }
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    Ok(())
}

pub fn begin(out: &mut Vec<u8>, kind: FourCc) -> usize {
    let at = out.len();
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&kind);
    at
}

pub fn end(out: &mut [u8], at: usize) -> Result<(), Error> {
    let len = u32::try_from(out.len() - at).map_err(|_| Error::TooLarge)?;
    out[at..at + 4].copy_from_slice(&len.to_be_bytes());
    Ok(())
}

pub fn write_box(out: &mut Vec<u8>, kind: FourCc, payload: &[u8]) -> Result<(), Error> {
    let len = u32::try_from(payload.len() + HEADER).map_err(|_| Error::TooLarge)?;
    out.reserve(payload.len() + HEADER);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(payload);
    Ok(())
}

pub fn write_description(out: &mut Vec<u8>, uuid: &Uuid, label: &str) -> Result<(), Error> {
    let at = begin(out, JUMD);
    out.extend_from_slice(uuid);
    out.push(toggles::REQUESTABLE | toggles::LABEL);
    put_cstr(out, label)?;
    end(out, at)
}

pub fn write_superbox(
    out: &mut Vec<u8>,
    uuid: &Uuid,
    label: &str,
    content: impl FnOnce(&mut Vec<u8>) -> Result<(), Error>,
) -> Result<usize, Error> {
    let at = begin(out, JUMB);
    write_description(out, uuid, label)?;
    content(out)?;
    end(out, at)?;
    Ok(at)
}

pub fn superbox_payload(out: &[u8], at: usize) -> &[u8] {
    &out[at + HEADER..]
}

#[cfg(test)]
mod tests;
