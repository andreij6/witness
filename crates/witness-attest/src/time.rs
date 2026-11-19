use crate::Error;
use crate::der::{Tag, Tlv};

pub const NS_PER_SEC: u64 = 1_000_000_000;
pub const NS_PER_MS: u64 = 1_000_000;

pub const fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn digits(s: &[u8]) -> Result<u32, Error> {
    s.iter().try_fold(0u32, |acc, &c| {
        if c.is_ascii_digit() {
            Ok(acc * 10 + u32::from(c - b'0'))
        } else {
            Err(Error::Time)
        }
    })
}

pub fn unix_seconds(year: i64, month: u32, day: u32, h: u32, m: u32, s: u32) -> Result<i64, Error> {
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || h > 23
        || m > 59
        || s > 59
    {
        return Err(Error::Time);
    }
    Ok(days_from_civil(year, month, day) * 86_400 + i64::from(h * 3600 + m * 60 + s))
}

pub fn parse_utc_time(v: &[u8]) -> Result<i64, Error> {
    if v.len() != 13 || v[12] != b'Z' {
        return Err(Error::Time);
    }
    let yy = i64::from(digits(&v[0..2])?);
    let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
    fields(year, &v[2..12])
}

pub fn parse_generalized_time(v: &[u8]) -> Result<i64, Error> {
    if v.len() != 15 || v[14] != b'Z' {
        return Err(Error::Time);
    }
    fields(i64::from(digits(&v[0..4])?), &v[4..14])
}

fn fields(year: i64, v: &[u8]) -> Result<i64, Error> {
    unix_seconds(
        year,
        digits(&v[0..2])?,
        digits(&v[2..4])?,
        digits(&v[4..6])?,
        digits(&v[6..8])?,
        digits(&v[8..10])?,
    )
}

pub fn parse_time(tlv: &Tlv<'_>) -> Result<i64, Error> {
    match tlv.tag {
        Tag::UTC_TIME => parse_utc_time(tlv.value),
        Tag::GENERALIZED_TIME => parse_generalized_time(tlv.value),
        _ => Err(Error::Time),
    }
}

pub const fn ns_to_secs(ns: u64) -> i64 {
    (ns / NS_PER_SEC) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{Rng, civil_from_days};

    #[test]
    fn epoch_is_zero() {
        assert_eq!(unix_seconds(1970, 1, 1, 0, 0, 0), Ok(0));
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn known_instants() {
        assert_eq!(parse_utc_time(b"160526162852Z"), Ok(1_464_280_132));
        assert_eq!(parse_utc_time(b"250717223218Z"), Ok(1_752_791_538));
        assert_eq!(
            parse_generalized_time(b"20380119031408Z"),
            Ok(2_147_483_648)
        );
        assert_eq!(
            parse_generalized_time(b"99991231235959Z"),
            Ok(253_402_300_799)
        );
        assert_eq!(parse_generalized_time(b"20000229000000Z"), Ok(951_782_400));
    }

    #[test]
    fn utc_time_century_pivot() {
        assert_eq!(
            parse_utc_time(b"500101000000Z"),
            unix_seconds(1950, 1, 1, 0, 0, 0)
        );
        assert_eq!(
            parse_utc_time(b"491231235959Z"),
            unix_seconds(2049, 12, 31, 23, 59, 59)
        );
        assert!(parse_utc_time(b"700101000000Z").unwrap() == 0);
    }

    #[test]
    fn pre_epoch_is_negative() {
        assert_eq!(parse_utc_time(b"691231235959Z"), Ok(-1));
        assert!(parse_generalized_time(b"16000301000000Z").unwrap() < 0);
    }

    #[test]
    fn rejects_malformed() {
        for bad in [
            &b"160526162852"[..],
            b"160526162852+",
            b"1605261628Z",
            b"16052616285aZ",
            b"161326162852Z",
            b"160001162852Z",
            b"160532162852Z",
            b"160526242852Z",
            b"160526166052Z",
            b"160526162860Z",
            b"230229000000Z",
            b"160631000000Z",
            b"",
            b"1605261628520Z",
        ] {
            assert_eq!(parse_utc_time(bad), Err(Error::Time), "{bad:?}");
        }
        for bad in [
            &b"20160526162852"[..],
            b"20160526162852.5Z",
            b"2016052616285Z",
            b"19000229000000Z",
            b"2016-526162852Z",
        ] {
            assert_eq!(parse_generalized_time(bad), Err(Error::Time), "{bad:?}");
        }
        assert!(parse_generalized_time(b"20000229000000Z").is_ok());
    }

    #[test]
    fn parse_time_dispatches_on_tag() {
        let utc = Tlv {
            tag: Tag::UTC_TIME,
            value: b"700101000001Z",
            raw: &[],
        };
        let gen_t = Tlv {
            tag: Tag::GENERALIZED_TIME,
            value: b"19700101000002Z",
            raw: &[],
        };
        let other = Tlv {
            tag: Tag::OCTET_STRING,
            value: b"700101000001Z",
            raw: &[],
        };
        assert_eq!(parse_time(&utc), Ok(1));
        assert_eq!(parse_time(&gen_t), Ok(2));
        assert_eq!(parse_time(&other), Err(Error::Time));
    }

    #[test]
    fn round_trips_random_instants() {
        let mut rng = Rng::new(7);
        for _ in 0..5000 {
            let days = rng.below(2_932_896) as i64 - 719_162;
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
            let secs = rng.below(86_400) as u32;
            let t = unix_seconds(y, m, d, secs / 3600, secs / 60 % 60, secs % 60).unwrap();
            assert_eq!(t, days * 86_400 + i64::from(secs));
        }
    }

    #[test]
    fn ns_to_secs_truncates() {
        assert_eq!(ns_to_secs(0), 0);
        assert_eq!(ns_to_secs(1_999_999_999), 1);
        assert_eq!(ns_to_secs(u64::MAX), 18_446_744_073);
    }
}
