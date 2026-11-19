use candid::CandidType;
use serde::Deserialize;
use serde_bytes::ByteBuf;
pub use witness_canister_common::{Allow, Removal, Shell};

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LateOpening {
    #[serde(with = "serde_bytes")]
    pub install_nonce: Vec<u8>,
    pub wall_ms: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Enrollment {
    pub chain: Vec<ByteBuf>,
    pub key_id: Option<ByteBuf>,
    pub late: Option<LateOpening>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Enrolled {
    #[serde(with = "serde_bytes")]
    pub key_id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub c2pa_cert: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub ca_cert: Vec<u8>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum EnrollResult {
    #[serde(rename = "ok")]
    Ok(Enrolled),
    #[serde(rename = "err")]
    Err(String),
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum EraseResult {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "err")]
    Err(String),
}

impl From<Result<(), String>> for EraseResult {
    fn from(r: Result<(), String>) -> Self {
        r.map_or_else(Self::Err, |()| Self::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode, Principal};

    #[test]
    fn erase_result_from_result() {
        assert_eq!(EraseResult::from(Ok(())), EraseResult::Ok);
        assert_eq!(
            EraseResult::from(Err("x".into())),
            EraseResult::Err("x".into())
        );
    }

    #[test]
    fn candid_round_trips() {
        let e = Enrollment {
            chain: vec![ByteBuf::from(vec![1, 2]), ByteBuf::from(vec![3])],
            key_id: Some(ByteBuf::from(vec![9; 16])),
            late: Some(LateOpening {
                install_nonce: vec![4; 8],
                wall_ms: 77,
            }),
        };
        let bytes = Encode!(&e).unwrap();
        assert_eq!(Decode!(&bytes, Enrollment).unwrap(), e);
        assert_eq!(Encode!(&e).unwrap(), bytes);
        let r = EnrollResult::Ok(Enrolled {
            key_id: vec![1],
            c2pa_cert: vec![2],
            ca_cert: vec![3],
        });
        assert_eq!(Decode!(&Encode!(&r).unwrap(), EnrollResult).unwrap(), r);
        let s = Shell {
            record_id: "A".into(),
            record_ix: 3,
            anchored_ns: 4,
            claim: vec![5],
            media_shard: 6,
            media: Principal::from_slice(&[7; 10]),
            dev: true,
        };
        assert_eq!(Decode!(&Encode!(&s).unwrap(), Shell).unwrap(), s);
    }

    #[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
    struct PagesShellInput {
        #[serde(with = "serde_bytes")]
        claim: Vec<u8>,
        media: Principal,
        media_shard: u8,
        anchored_ns: u64,
        dev: bool,
    }

    #[test]
    fn shell_decodes_as_pages_shell_input() {
        let s = Shell {
            record_id: "A".into(),
            record_ix: 3,
            anchored_ns: 4,
            claim: vec![5, 6],
            media_shard: 1,
            media: Principal::from_slice(&[8; 10]),
            dev: false,
        };
        let got = Decode!(&Encode!(&vec![s.clone()]).unwrap(), Vec<PagesShellInput>).unwrap();
        assert_eq!(
            got,
            vec![PagesShellInput {
                claim: s.claim,
                media: s.media,
                media_shard: 1,
                anchored_ns: 4,
                dev: false,
            }]
        );
    }

    #[test]
    fn allow_and_removal_round_trip() {
        let a = Allow::Key {
            key_id: ByteBuf::from(vec![1; 16]),
            pubkey: ByteBuf::from(vec![2; 91]),
        };
        assert_eq!(Decode!(&Encode!(&a).unwrap(), Allow).unwrap(), a);
        assert_eq!(
            Decode!(&Encode!(&Removal::Erased).unwrap(), Removal).unwrap(),
            Removal::Erased
        );
        assert!(Decode!(&Encode!(&a).unwrap(), Removal).is_err());
    }

    #[test]
    fn decoding_wrong_type_fails() {
        let bytes = Encode!(&EraseResult::Ok).unwrap();
        assert!(Decode!(&bytes, Enrollment).is_err());
        assert!(Decode!(b"garbage", EraseResult).is_err());
    }
}
