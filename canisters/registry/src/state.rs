use candid::{CandidType, Decode, Encode};
use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use std::borrow::Cow;

#[derive(CandidType, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Published {
    pub leaf_count: u64,
    pub root: ByteBuf,
    pub at_ns: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub secret: Option<ByteBuf>,
    pub ca_root: Option<ByteBuf>,
    pub next_serial: u64,
    pub published: Option<Published>,
}

impl State {
    pub fn secret(&self) -> Option<[u8; 32]> {
        self.secret
            .as_ref()
            .and_then(|s| s.as_slice().try_into().ok())
    }

    pub fn ca_root(&self) -> Option<&[u8]> {
        self.ca_root.as_deref().map(|b| b.as_slice())
    }

    pub fn take_serial(&mut self) -> u64 {
        let s = self.next_serial.max(crate::config::ROOT_SERIAL + 1);
        self.next_serial = s + 1;
        s
    }
}

impl Storable for State {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(Encode!(self).expect("candid encode"))
    }

    fn into_bytes(self) -> Vec<u8> {
        Encode!(&self).expect("candid encode")
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("candid decode")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let s = State::default();
        assert_eq!(s.secret(), None);
        assert_eq!(s.ca_root(), None);
    }

    #[test]
    fn secret_requires_32_bytes() {
        let mut s = State {
            secret: Some(ByteBuf::from(vec![1; 31])),
            ..State::default()
        };
        assert_eq!(s.secret(), None);
        s.secret = Some(ByteBuf::from(vec![1; 32]));
        assert_eq!(s.secret(), Some([1; 32]));
    }

    #[test]
    fn serials_start_after_root_and_increase() {
        let mut s = State::default();
        assert_eq!(s.take_serial(), 2);
        assert_eq!(s.take_serial(), 3);
        s.next_serial = 1;
        assert_eq!(s.take_serial(), 2);
    }

    #[test]
    fn storable_round_trips() {
        let s = State {
            secret: Some(ByteBuf::from(vec![7; 32])),
            ca_root: Some(ByteBuf::from(vec![1, 2, 3])),
            next_serial: 9,
            published: Some(Published {
                leaf_count: 4,
                root: ByteBuf::from(vec![5; 32]),
                at_ns: 6,
            }),
        };
        assert_eq!(State::from_bytes(s.to_bytes()), s);
        assert_eq!(State::from_bytes(Cow::Owned(s.clone().into_bytes())), s);
        assert_eq!(s.to_bytes(), s.clone().into_bytes());
    }

    #[test]
    #[should_panic(expected = "candid decode")]
    fn storable_rejects_garbage() {
        State::from_bytes(Cow::Borrowed(b"nope"));
    }
}
