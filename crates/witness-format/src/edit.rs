use crate::cbor::{Decode, Decoder, Encode, Encoder, Error};

pub const MAX_ROTATE_DEG_E2: i16 = 1000;
pub const MAX_ORIENT: u8 = 7;

code_enum! {
    pub enum EditKind {
        Resize = 0,
        Recompress = 1,
        Orient = 2,
        Crop = 3,
        Rotate = 4,
        Tone = 5,
        Filter = 6,
        Blur = 7,
    }
}

code_enum! {
    pub enum EditClass {
        Presentation = 0,
        Global = 1,
        LocalDeclared = 2,
    }
}

code_enum! {
    pub enum BlurClass {
        Face = 0,
        Plate = 1,
        HouseNo = 2,
        Other = 3,
    }
}

code_enum! {
    pub enum ImageFormat {
        Jpeg = 0,
        Png = 1,
        Webp = 2,
        Avif = 3,
        Heic = 4,
    }
}

impl EditKind {
    pub const fn class(self) -> EditClass {
        match self {
            Self::Resize | Self::Recompress | Self::Orient => EditClass::Presentation,
            Self::Crop | Self::Rotate | Self::Tone | Self::Filter => EditClass::Global,
            Self::Blur => EditClass::LocalDeclared,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn is_valid(&self) -> bool {
        self.w > 0
            && self.h > 0
            && self.x.checked_add(self.w).is_some()
            && self.y.checked_add(self.h).is_some()
    }

    pub const fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }
}

impl Encode for Rect {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(4);
        e.field(0, &self.x);
        e.field(1, &self.y);
        e.field(2, &self.w);
        e.field(3, &self.h);
    }
}

impl<'a> Decode<'a> for Rect {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let r = Self {
            x: f.req(d, 0)?,
            y: f.req(d, 1)?,
            w: f.req(d, 2)?,
            h: f.req(d, 3)?,
        };
        f.finish(d)?;
        if r.is_valid() {
            Ok(r)
        } else {
            Err(Error::Invalid)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edit {
    Resize {
        w: u32,
        h: u32,
        filter: u8,
    },
    Recompress {
        format: ImageFormat,
        quality: u8,
    },
    Orient {
        orientation: u8,
    },
    Crop(Rect),
    Rotate {
        deg_e2: i16,
    },
    Tone {
        exposure_e2: i16,
        contrast_e2: i16,
        saturation_e2: i16,
    },
    Filter {
        id: u16,
        strength_e2: u16,
    },
    Blur {
        rect: Rect,
        class: BlurClass,
        sigma_e2: u16,
    },
}

impl Edit {
    pub const fn kind(&self) -> EditKind {
        match self {
            Self::Resize { .. } => EditKind::Resize,
            Self::Recompress { .. } => EditKind::Recompress,
            Self::Orient { .. } => EditKind::Orient,
            Self::Crop(_) => EditKind::Crop,
            Self::Rotate { .. } => EditKind::Rotate,
            Self::Tone { .. } => EditKind::Tone,
            Self::Filter { .. } => EditKind::Filter,
            Self::Blur { .. } => EditKind::Blur,
        }
    }

    pub const fn class(&self) -> EditClass {
        self.kind().class()
    }

    pub const fn is_valid(&self) -> bool {
        match *self {
            Self::Resize { w, h, .. } => w > 0 && h > 0,
            Self::Recompress { quality, .. } => quality <= 100,
            Self::Orient { orientation } => orientation <= MAX_ORIENT,
            Self::Crop(r) | Self::Blur { rect: r, .. } => r.is_valid(),
            Self::Rotate { deg_e2 } => deg_e2 >= -MAX_ROTATE_DEG_E2 && deg_e2 <= MAX_ROTATE_DEG_E2,
            Self::Tone { .. } | Self::Filter { .. } => true,
        }
    }

    fn param_count(&self) -> usize {
        match self {
            Self::Orient { .. } | Self::Crop(_) | Self::Rotate { .. } => 1,
            Self::Recompress { .. } | Self::Filter { .. } => 2,
            Self::Resize { .. } | Self::Tone { .. } | Self::Blur { .. } => 3,
        }
    }

    fn encode_params(&self, e: &mut Encoder<'_>) {
        e.map(self.param_count());
        match self {
            Self::Resize { w, h, filter } => {
                e.field(0, w);
                e.field(1, h);
                e.field(2, filter);
            }
            Self::Recompress { format, quality } => {
                e.field(0, format);
                e.field(1, quality);
            }
            Self::Orient { orientation } => e.field(0, orientation),
            Self::Crop(r) => e.field(0, r),
            Self::Rotate { deg_e2 } => e.field(0, deg_e2),
            Self::Tone {
                exposure_e2,
                contrast_e2,
                saturation_e2,
            } => {
                e.field(0, exposure_e2);
                e.field(1, contrast_e2);
                e.field(2, saturation_e2);
            }
            Self::Filter { id, strength_e2 } => {
                e.field(0, id);
                e.field(1, strength_e2);
            }
            Self::Blur {
                rect,
                class,
                sigma_e2,
            } => {
                e.field(0, rect);
                e.field(1, class);
                e.field(2, sigma_e2);
            }
        }
    }

    fn decode_params(kind: EditKind, d: &mut Decoder<'_>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let edit = match kind {
            EditKind::Resize => Self::Resize {
                w: f.req(d, 0)?,
                h: f.req(d, 1)?,
                filter: f.req(d, 2)?,
            },
            EditKind::Recompress => Self::Recompress {
                format: f.req(d, 0)?,
                quality: f.req(d, 1)?,
            },
            EditKind::Orient => Self::Orient {
                orientation: f.req(d, 0)?,
            },
            EditKind::Crop => Self::Crop(f.req(d, 0)?),
            EditKind::Rotate => Self::Rotate {
                deg_e2: f.req(d, 0)?,
            },
            EditKind::Tone => Self::Tone {
                exposure_e2: f.req(d, 0)?,
                contrast_e2: f.req(d, 1)?,
                saturation_e2: f.req(d, 2)?,
            },
            EditKind::Filter => Self::Filter {
                id: f.req(d, 0)?,
                strength_e2: f.req(d, 1)?,
            },
            EditKind::Blur => Self::Blur {
                rect: f.req(d, 0)?,
                class: f.req(d, 1)?,
                sigma_e2: f.req(d, 2)?,
            },
        };
        f.finish(d)?;
        Ok(edit)
    }
}

impl Encode for Edit {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(2);
        e.field(0, &self.kind());
        e.u64(1);
        self.encode_params(e);
    }
}

impl<'a> Decode<'a> for Edit {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let kind: EditKind = f.req(d, 0)?;
        if !f.key(d, 1)? {
            return Err(Error::MissingKey(1));
        }
        let edit = Self::decode_params(kind, d)?;
        f.finish(d)?;
        if edit.is_valid() {
            Ok(edit)
        } else {
            Err(Error::Invalid)
        }
    }
}

pub fn presentation_only(edits: &[Edit]) -> bool {
    edits.iter().all(|e| e.class() == EditClass::Presentation)
}

pub fn has_local(edits: &[Edit]) -> bool {
    edits.iter().any(|e| e.class() == EditClass::LocalDeclared)
}

#[cfg(test)]
mod tests;
