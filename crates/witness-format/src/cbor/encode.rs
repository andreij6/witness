use super::Encode;
use super::decode::{
    MAJOR_ARRAY, MAJOR_BYTES, MAJOR_MAP, MAJOR_NINT, MAJOR_SIMPLE, MAJOR_TEXT, MAJOR_UINT,
};
use alloc::vec::Vec;

pub struct Encoder<'b> {
    buf: &'b mut Vec<u8>,
}

impl<'b> Encoder<'b> {
    pub fn new(buf: &'b mut Vec<u8>) -> Self {
        Self { buf }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn head(&mut self, major: u8, arg: u64) {
        let m = major << 5;
        if arg < 24 {
            self.buf.push(m | arg as u8);
        } else if arg <= 0xff {
            self.buf.extend_from_slice(&[m | 24, arg as u8]);
        } else if arg <= 0xffff {
            self.buf.push(m | 25);
            self.buf.extend_from_slice(&(arg as u16).to_be_bytes());
        } else if arg <= 0xffff_ffff {
            self.buf.push(m | 26);
            self.buf.extend_from_slice(&(arg as u32).to_be_bytes());
        } else {
            self.buf.push(m | 27);
            self.buf.extend_from_slice(&arg.to_be_bytes());
        }
    }

    pub fn u64(&mut self, v: u64) {
        self.head(MAJOR_UINT, v);
    }

    pub fn nint(&mut self, n: u64) {
        self.head(MAJOR_NINT, n);
    }

    pub fn i64(&mut self, v: i64) {
        if v >= 0 {
            self.u64(v as u64);
        } else {
            self.nint(!(v as u64));
        }
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.head(MAJOR_BYTES, b.len() as u64);
        self.buf.extend_from_slice(b);
    }

    pub fn text(&mut self, s: &str) {
        self.head(MAJOR_TEXT, s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn bool(&mut self, v: bool) {
        self.head(MAJOR_SIMPLE, if v { 21 } else { 20 });
    }

    pub fn null(&mut self) {
        self.head(MAJOR_SIMPLE, 22);
    }

    pub fn array(&mut self, len: usize) {
        self.head(MAJOR_ARRAY, len as u64);
    }

    pub fn map(&mut self, len: usize) {
        self.head(MAJOR_MAP, len as u64);
    }

    pub fn raw(&mut self, item: &[u8]) {
        self.buf.extend_from_slice(item);
    }

    pub fn put<T: Encode + ?Sized>(&mut self, v: &T) {
        v.encode(self);
    }

    pub fn field<T: Encode + ?Sized>(&mut self, key: u64, v: &T) {
        self.u64(key);
        v.encode(self);
    }

    pub fn opt_field<T: Encode>(&mut self, key: u64, v: Option<&T>) {
        if let Some(v) = v {
            self.field(key, v);
        }
    }
}
