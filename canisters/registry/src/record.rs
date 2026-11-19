use witness_format::RecordId;
use witness_format::crypto::sha256;

pub const LEAF_DATA_LEN: usize = 32 * 3 + 8;
const HEADER: usize = 8 + 8 + 1 + 1 + 4 + 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordEntry {
    pub anchored_ns: u64,
    pub root_ix: u64,
    pub index_shard: u8,
    pub media_shard: u8,
    pub claim: Vec<u8>,
    pub c2pa_claim: Vec<u8>,
    pub cose: Vec<u8>,
}

pub fn leaf_data(
    claim: &[u8],
    c2pa_claim: &[u8],
    cose: &[u8],
    anchored_ns: u64,
) -> [u8; LEAF_DATA_LEN] {
    let mut out = [0u8; LEAF_DATA_LEN];
    out[..32].copy_from_slice(&sha256(claim));
    out[32..64].copy_from_slice(&sha256(c2pa_claim));
    out[64..96].copy_from_slice(&sha256(cose));
    out[96..].copy_from_slice(&anchored_ns.to_be_bytes());
    out
}

fn len32(n: usize) -> [u8; 4] {
    u32::try_from(n).unwrap_or(u32::MAX).to_be_bytes()
}

impl RecordEntry {
    pub fn record_id(&self) -> RecordId {
        RecordId::from_claim_cbor(&self.claim)
    }

    pub fn leaf_data(&self) -> [u8; LEAF_DATA_LEN] {
        leaf_data(&self.claim, &self.c2pa_claim, &self.cose, self.anchored_ns)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(HEADER + self.claim.len() + self.c2pa_claim.len() + self.cose.len());
        out.extend_from_slice(&self.anchored_ns.to_be_bytes());
        out.extend_from_slice(&self.root_ix.to_be_bytes());
        out.push(self.index_shard);
        out.push(self.media_shard);
        out.extend_from_slice(&len32(self.claim.len()));
        out.extend_from_slice(&len32(self.c2pa_claim.len()));
        out.extend_from_slice(&self.claim);
        out.extend_from_slice(&self.c2pa_claim);
        out.extend_from_slice(&self.cose);
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let (head, body) = bytes.split_first_chunk::<HEADER>()?;
        let u64_at = |i: usize| u64::from_be_bytes(head[i..i + 8].try_into().unwrap_or_default());
        let u32_at = |i: usize| {
            usize::try_from(u32::from_be_bytes(
                head[i..i + 4].try_into().unwrap_or_default(),
            ))
            .ok()
        };
        let claim_len = u32_at(18)?;
        let c2pa_len = u32_at(22)?;
        let (claim, rest) = body.split_at_checked(claim_len)?;
        let (c2pa_claim, cose) = rest.split_at_checked(c2pa_len)?;
        Some(Self {
            anchored_ns: u64_at(0),
            root_ix: u64_at(8),
            index_shard: head[16],
            media_shard: head[17],
            claim: claim.to_vec(),
            c2pa_claim: c2pa_claim.to_vec(),
            cose: cose.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> RecordEntry {
        RecordEntry {
            anchored_ns: 0x0102_0304_0506_0708,
            root_ix: 42,
            index_shard: 3,
            media_shard: 250,
            claim: vec![1, 2, 3],
            c2pa_claim: vec![4; 300],
            cose: vec![5, 6],
        }
    }

    #[test]
    fn round_trips() {
        let e = entry();
        let bytes = e.encode();
        assert_eq!(bytes.len(), HEADER + 3 + 300 + 2);
        assert_eq!(RecordEntry::decode(&bytes), Some(e));
    }

    #[test]
    fn round_trips_empty_parts() {
        let e = RecordEntry {
            claim: vec![],
            c2pa_claim: vec![],
            cose: vec![],
            ..entry()
        };
        assert_eq!(RecordEntry::decode(&e.encode()), Some(e));
    }

    #[test]
    fn rejects_truncation_everywhere_before_cose() {
        let bytes = entry().encode();
        for cut in 0..HEADER + 3 + 300 {
            assert_eq!(RecordEntry::decode(&bytes[..cut]), None, "cut {cut}");
        }
    }

    #[test]
    fn rejects_lengths_past_end() {
        let mut bytes = entry().encode();
        bytes[18..22].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(RecordEntry::decode(&bytes), None);
        let mut bytes = entry().encode();
        bytes[22..26].copy_from_slice(&10_000u32.to_be_bytes());
        assert_eq!(RecordEntry::decode(&bytes), None);
    }

    #[test]
    fn encoding_is_deterministic() {
        assert_eq!(entry().encode(), entry().encode());
    }

    #[test]
    fn leaf_data_binds_every_part() {
        let e = entry();
        let base = e.leaf_data();
        assert_eq!(
            base,
            leaf_data(&e.claim, &e.c2pa_claim, &e.cose, e.anchored_ns)
        );
        assert_eq!(&base[96..], &e.anchored_ns.to_be_bytes());
        let variants = [
            RecordEntry {
                claim: vec![9],
                ..entry()
            },
            RecordEntry {
                c2pa_claim: vec![9],
                ..entry()
            },
            RecordEntry {
                cose: vec![9],
                ..entry()
            },
            RecordEntry {
                anchored_ns: 1,
                ..entry()
            },
        ];
        for v in variants {
            assert_ne!(v.leaf_data(), base);
        }
        assert_eq!(
            RecordEntry {
                root_ix: 7,
                media_shard: 1,
                ..entry()
            }
            .leaf_data(),
            base
        );
    }

    #[test]
    fn leaf_data_separates_part_boundaries() {
        assert_ne!(
            leaf_data(b"ab", b"c", b"", 0),
            leaf_data(b"a", b"bc", b"", 0)
        );
    }

    #[test]
    fn record_id_is_claim_hash() {
        let e = entry();
        assert_eq!(e.record_id(), RecordId::from_claim_cbor(&[1, 2, 3]));
    }
}
