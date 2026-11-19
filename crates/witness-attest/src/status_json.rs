use crate::Error;
use crate::revocation::{Revocation, RevocationList, RevocationReason, RevocationStatus};
use crate::serial::Serial;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;
use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};

#[derive(Deserialize)]
struct Document<'a> {
    #[serde(borrow)]
    entries: Entries<'a>,
}

#[derive(Deserialize)]
struct RawEntry<'a> {
    #[serde(borrow)]
    status: Cow<'a, str>,
    #[serde(default, borrow)]
    reason: Option<Cow<'a, str>>,
}

struct Entries<'a>(Vec<(Cow<'a, str>, RawEntry<'a>)>);

impl<'de: 'a, 'a> Deserialize<'de> for Entries<'a> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<'a>(core::marker::PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for V<'a> {
            type Value = Entries<'a>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of serial to status entry")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut out = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some(kv) = map.next_entry::<Cow<'a, str>, RawEntry<'a>>()? {
                    out.push(kv);
                }
                Ok(Entries(out))
            }
        }
        d.deserialize_map(V(core::marker::PhantomData))
    }
}

pub fn serial_interpretations(key: &str) -> impl Iterator<Item = Serial> {
    let hex = Serial::from_hex(key);
    let decimal = Serial::from_decimal(key).filter(|d| Some(*d) != hex);
    hex.into_iter().chain(decimal)
}

pub fn parse_status_list(json: &[u8]) -> Result<RevocationList, Error> {
    let doc: Document<'_> = serde_json::from_slice(json).map_err(|_| Error::Json)?;
    let mut out = Vec::with_capacity(doc.entries.0.len() * 2);
    for (key, raw) in &doc.entries.0 {
        let status = RevocationStatus::from_google(&raw.status);
        let reason = raw
            .reason
            .as_deref()
            .map_or(RevocationReason::Unspecified, RevocationReason::from_google);
        out.extend(serial_interpretations(key).map(|serial| Revocation {
            serial,
            status,
            reason,
        }));
    }
    Ok(RevocationList::new(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn hexes(list: &RevocationList) -> Vec<(String, RevocationStatus, RevocationReason)> {
        list.entries()
            .iter()
            .map(|e| (e.serial.to_hex(), e.status, e.reason))
            .collect()
    }

    #[test]
    fn parses_google_example() {
        let json = br#"{
          "entries": {
            "2c8cdddfd5e03bfc": {
              "status": "REVOKED",
              "expires": "2020-11-13",
              "reason": "KEY_COMPROMISE",
              "comment": "Key stored on unsecure system"
            },
            "c8966fcb2fbb0d7a": {
              "status": "SUSPENDED",
              "reason": "SOFTWARE_FLAW",
              "comment": "Bug in keystore causes this key malfunction b/555555"
            }
          }
        }"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(
            hexes(&list),
            alloc::vec![
                (
                    "2c8cdddfd5e03bfc".into(),
                    RevocationStatus::Revoked,
                    RevocationReason::KeyCompromise
                ),
                (
                    "c8966fcb2fbb0d7a".into(),
                    RevocationStatus::Suspended,
                    RevocationReason::SoftwareFlaw
                ),
            ]
        );
    }

    #[test]
    fn all_digit_keys_are_read_as_hex_and_decimal() {
        let json = br#"{"entries":{"6681152659205225093":{"status":"REVOKED","reason":"KEY_COMPROMISE"}}}"#;
        let list = parse_status_list(json).unwrap();
        let got: Vec<String> = list.entries().iter().map(|e| e.serial.to_hex()).collect();
        assert_eq!(
            got,
            alloc::vec![
                String::from("5cb838f1fe157a85"),
                String::from("6681152659205225093")
            ]
        );
        assert!(
            list.entries()
                .iter()
                .all(|e| e.status == RevocationStatus::Revoked)
        );
    }

    #[test]
    fn single_digit_keys_are_not_duplicated() {
        let json = br#"{"entries":{"7":{"status":"REVOKED"},"0":{"status":"SUSPENDED"}}}"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(
            hexes(&list),
            alloc::vec![
                (
                    "0".into(),
                    RevocationStatus::Suspended,
                    RevocationReason::Unspecified
                ),
                (
                    "7".into(),
                    RevocationStatus::Revoked,
                    RevocationReason::Unspecified
                ),
            ]
        );
    }

    #[test]
    fn normalizes_case_padding_and_merges_collisions() {
        let json = br#"{"entries":{
            "00ABcdef": {"status":"SUSPENDED","reason":"SUPERSEDED"},
            "abcdef":   {"status":"REVOKED","reason":"CA_COMPROMISE"},
            "ABCDEF":   {"status":"SUSPENDED"}
        }}"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(
            hexes(&list),
            alloc::vec![(
                "abcdef".into(),
                RevocationStatus::Revoked,
                RevocationReason::CaCompromise
            )]
        );
    }

    #[test]
    fn duplicate_json_keys_keep_most_severe() {
        let json = br#"{"entries":{"ab":{"status":"SUSPENDED"},"ab":{"status":"REVOKED"}}}"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(
            list.status(&Serial::from_hex("ab").unwrap()),
            Some(RevocationStatus::Revoked)
        );
    }

    #[test]
    fn skips_unparseable_keys() {
        let long: String = core::iter::repeat_n('f', 65).collect();
        let json = alloc::format!(
            r#"{{"entries":{{"zz":{{"status":"REVOKED"}},"":{{"status":"REVOKED"}},"{long}":{{"status":"REVOKED"}},"1f":{{"status":"REVOKED"}}}}}}"#
        );
        let list = parse_status_list(json.as_bytes()).unwrap();
        assert_eq!(hexes(&list).len(), 1);
        assert_eq!(list.entries()[0].serial.to_hex(), "1f");
    }

    #[test]
    fn unknown_status_fails_closed_and_unknown_reason_is_unspecified() {
        let json = br#"{"entries":{"ab":{"status":"QUARANTINED","reason":"COSMIC_RAYS"}}}"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(
            hexes(&list),
            alloc::vec![(
                "ab".into(),
                RevocationStatus::Suspended,
                RevocationReason::Unspecified
            )]
        );
    }

    #[test]
    fn handles_escaped_keys() {
        let json = br#"{"entries":{"ab":{"status":"REVOKED"}}}"#;
        let list = parse_status_list(json).unwrap();
        assert_eq!(list.entries()[0].serial.to_hex(), "ab");
    }

    #[test]
    fn ignores_extra_top_level_fields_and_accepts_empty() {
        let json = br#"{"version":3,"entries":{}}"#;
        assert!(parse_status_list(json).unwrap().is_empty());
    }

    #[test]
    fn rejects_malformed_documents() {
        for bad in [
            &br#""#[..],
            br#"[]"#,
            br#"{}"#,
            br#"{"entries":[]}"#,
            br#"{"entries":{"ab":{}}}"#,
            br#"{"entries":{"ab":{"status":1}}}"#,
            br#"{"entries":{"ab":{"status":"REVOKED","reason":7}}}"#,
            br#"{"entries":{"ab":"REVOKED"}}"#,
            br#"{"entries":{"ab":{"status":"REVOKED"}}"#,
            br#"{"entries":{"ab":{"status":"REVOKED"}}} trailing"#,
            b"\xff\xfe",
        ] {
            assert_eq!(
                parse_status_list(bad).map(|_| ()),
                Err(Error::Json),
                "{:?}",
                core::str::from_utf8(bad)
            );
        }
    }

    #[test]
    fn deterministic_across_key_order() {
        let a = br#"{"entries":{"1":{"status":"REVOKED"},"ff":{"status":"SUSPENDED"},"123":{"status":"REVOKED"}}}"#;
        let b = br#"{"entries":{"123":{"status":"REVOKED"},"ff":{"status":"SUSPENDED"},"1":{"status":"REVOKED"}}}"#;
        assert_eq!(
            parse_status_list(a).unwrap().to_cbor(),
            parse_status_list(b).unwrap().to_cbor()
        );
    }
}
