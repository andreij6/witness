use crate::Error;
use minicbor::Decoder;
use minicbor::data::Type;

pub fn map_len(d: &mut Decoder<'_>) -> Result<u64, Error> {
    d.map()?.ok_or(Error::MalformedCbor)
}

pub fn array_len(d: &mut Decoder<'_>) -> Result<u64, Error> {
    let n = d.array()?.ok_or(Error::MalformedCbor)?;
    if n > (d.input().len() - d.position()) as u64 {
        return Err(Error::MalformedCbor);
    }
    Ok(n)
}

pub fn whole<'a, T>(
    bytes: &'a [u8],
    f: impl FnOnce(&mut Decoder<'a>) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut d = Decoder::new(bytes);
    let v = f(&mut d)?;
    if d.position() != bytes.len() {
        return Err(Error::MalformedCbor);
    }
    Ok(v)
}

pub fn set_once<T>(slot: &mut Option<T>, v: T, dup: Error) -> Result<(), Error> {
    match slot.replace(v) {
        None => Ok(()),
        Some(_) => Err(dup),
    }
}

pub fn is_null(d: &Decoder<'_>) -> Result<bool, Error> {
    Ok(d.datatype()? == Type::Null)
}

pub fn int_key(d: &mut Decoder<'_>) -> Result<i64, Error> {
    match d.datatype()? {
        Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64 => Ok(d.i64()?),
        _ => Err(Error::MalformedCbor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use witness_format::cbor::Encoder;

    fn enc(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
        let mut v = Vec::new();
        f(&mut Encoder::new(&mut v));
        v
    }

    #[test]
    fn map_len_rejects_indefinite() {
        assert_eq!(
            map_len(&mut Decoder::new(&[0xbf, 0xff])),
            Err(Error::MalformedCbor)
        );
        assert_eq!(map_len(&mut Decoder::new(&[0xa2])), Ok(2));
        assert_eq!(
            map_len(&mut Decoder::new(&[0x80])),
            Err(Error::MalformedCbor)
        );
    }

    #[test]
    fn array_len_bounds_by_remaining_input() {
        assert_eq!(
            array_len(&mut Decoder::new(&[0x9f, 0xff])),
            Err(Error::MalformedCbor)
        );
        assert_eq!(array_len(&mut Decoder::new(&[0x82, 1, 2])), Ok(2));
        let huge = enc(|e| e.array(1 << 40));
        assert_eq!(
            array_len(&mut Decoder::new(&huge)),
            Err(Error::MalformedCbor)
        );
    }

    #[test]
    fn whole_rejects_trailing() {
        assert_eq!(whole(&[0x01], |d| Ok(d.u8()?)), Ok(1));
        assert_eq!(
            whole(&[0x01, 0x02], |d| Ok(d.u8()?)),
            Err(Error::MalformedCbor)
        );
        assert_eq!(whole(&[], |d| Ok(d.u8()?)), Err(Error::MalformedCbor));
    }

    #[test]
    fn set_once_detects_duplicates() {
        let mut slot = None;
        assert!(set_once(&mut slot, 1, Error::MalformedClaim).is_ok());
        assert_eq!(
            set_once(&mut slot, 2, Error::MalformedClaim),
            Err(Error::MalformedClaim)
        );
    }

    #[test]
    fn int_key_accepts_signed_and_unsigned_only() {
        let neg = enc(|e| e.i64(-7));
        assert_eq!(int_key(&mut Decoder::new(&neg)), Ok(-7));
        assert_eq!(int_key(&mut Decoder::new(&[0x18, 33])), Ok(33));
        let text = enc(|e| e.text("alg"));
        assert_eq!(int_key(&mut Decoder::new(&text)), Err(Error::MalformedCbor));
        assert_eq!(int_key(&mut Decoder::new(&[])), Err(Error::MalformedCbor));
    }

    #[test]
    fn is_null_detects_null() {
        assert_eq!(is_null(&Decoder::new(&[0xf6])), Ok(true));
        assert_eq!(is_null(&Decoder::new(&[0x40])), Ok(false));
    }
}
