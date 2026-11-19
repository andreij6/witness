use crate::index::Posting;
use crate::media::MediaErr;
use candid::{CandidType, Principal, define_service, func};
use serde::Deserialize;
use serde_bytes::ByteBuf;

pub const PUT: &str = "put";
pub const ERASE: &str = "erase";
pub const PUT_SHELL: &str = "put_shell";
pub const TOMBSTONE: &str = "tombstone";
pub const SYNC: &str = "sync";

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shell {
    pub record_id: String,
    pub record_ix: u64,
    pub anchored_ns: u64,
    #[serde(with = "serde_bytes")]
    pub claim: Vec<u8>,
    pub media_shard: u8,
    pub media: Principal,
    pub dev: bool,
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Removal {
    #[serde(rename = "erased")]
    Erased,
    #[serde(rename = "takedown")]
    Takedown,
    #[serde(rename = "revoked")]
    Revoked,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PageErr {
    #[serde(rename = "malformed")]
    Malformed(String),
    #[serde(rename = "gone")]
    Gone,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PageResult {
    #[serde(rename = "ok")]
    Ok(String),
    #[serde(rename = "err")]
    Err(PageErr),
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Allow {
    #[serde(rename = "key")]
    Key { key_id: ByteBuf, pubkey: ByteBuf },
    #[serde(rename = "record")]
    Record { record_id: String, key_id: ByteBuf },
    #[serde(rename = "revoke")]
    Revoke(ByteBuf),
}

define_service!(pub IndexPeer : {
    "put" : func!((Vec<Posting>) -> ());
    "erase" : func!((u64) -> ());
});

define_service!(pub PagesPeer : {
    "put_shell" : func!((Vec<Shell>) -> (Vec<PageResult>));
    "tombstone" : func!((Vec<String>, Removal) -> (Vec<PageResult>));
});

define_service!(pub MediaPeer : {
    "sync" : func!((Vec<Allow>) -> (u32));
    "erase" : func!((String) -> (Result<u32, MediaErr>));
});

pub fn peer_did(service: &str) -> String {
    format!("{}\nservice : {service}\n", crate::DID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    fn shell() -> Shell {
        Shell {
            record_id: "0123456789ABCDEF".into(),
            record_ix: 3,
            anchored_ns: 4,
            claim: vec![5, 6],
            media_shard: 1,
            media: Principal::from_slice(&[8; 10]),
            dev: true,
        }
    }

    #[test]
    fn shell_round_trips_and_is_deterministic() {
        let s = shell();
        let bytes = Encode!(&s).unwrap();
        assert_eq!(Decode!(&bytes, Shell).unwrap(), s);
        assert_eq!(Encode!(&s).unwrap(), bytes);
    }

    #[test]
    fn removal_and_results_round_trip() {
        for r in [Removal::Erased, Removal::Takedown, Removal::Revoked] {
            assert_eq!(Decode!(&Encode!(&r).unwrap(), Removal).unwrap(), r);
        }
        for p in [
            PageResult::Ok("x".into()),
            PageResult::Err(PageErr::Gone),
            PageResult::Err(PageErr::Malformed("m".into())),
        ] {
            assert_eq!(Decode!(&Encode!(&p).unwrap(), PageResult).unwrap(), p);
        }
    }

    #[test]
    fn allow_variants_round_trip_and_reject_other_types() {
        let all = [
            Allow::Key {
                key_id: ByteBuf::from(vec![1; 16]),
                pubkey: ByteBuf::from(vec![2; 91]),
            },
            Allow::Record {
                record_id: "R".into(),
                key_id: ByteBuf::from(vec![3; 16]),
            },
            Allow::Revoke(ByteBuf::from(vec![4; 16])),
        ];
        for a in &all {
            assert_eq!(&Decode!(&Encode!(a).unwrap(), Allow).unwrap(), a);
            assert!(Decode!(&Encode!(a).unwrap(), Removal).is_err());
        }
        assert!(Decode!(b"garbage", Allow).is_err());
    }

    #[derive(CandidType, Deserialize)]
    enum OldAllow {
        #[serde(rename = "key")]
        Key { key_id: ByteBuf, pubkey: ByteBuf },
        #[serde(rename = "record")]
        Record { record_id: String, key_id: ByteBuf },
    }

    #[test]
    fn allow_reads_values_written_without_revoke() {
        let old = OldAllow::Record {
            record_id: "R".into(),
            key_id: ByteBuf::from(vec![9; 16]),
        };
        assert_eq!(
            Decode!(&Encode!(&old).unwrap(), Allow).unwrap(),
            Allow::Record {
                record_id: "R".into(),
                key_id: ByteBuf::from(vec![9; 16]),
            }
        );
        let old = OldAllow::Key {
            key_id: ByteBuf::from(vec![1]),
            pubkey: ByteBuf::from(vec![2]),
        };
        assert!(matches!(
            Decode!(&Encode!(&old).unwrap(), Allow).unwrap(),
            Allow::Key { .. }
        ));
    }

    #[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
    enum ErasedOnly {
        #[serde(rename = "erased")]
        Erased,
    }

    #[test]
    fn removal_is_wire_compatible_with_a_narrower_sender() {
        let bytes = Encode!(&ErasedOnly::Erased).unwrap();
        assert_eq!(Decode!(&bytes, Removal).unwrap(), Removal::Erased);
    }

    #[test]
    fn peer_did_names_the_service() {
        let did = peer_did("PagesPeer");
        assert!(did.ends_with("service : PagesPeer\n"));
        assert!(
            candid_parser::utils::CandidSource::Text(&did)
                .load()
                .is_ok()
        );
    }

    #[test]
    fn method_names() {
        assert_eq!(
            [PUT, ERASE, PUT_SHELL, TOMBSTONE, SYNC],
            ["put", "erase", "put_shell", "tombstone", "sync"]
        );
    }
}
