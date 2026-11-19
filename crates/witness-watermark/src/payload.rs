use crate::bch::{self, PAD_BITS, PARITY_BITS, PARITY_MASK, T};
use witness_format::CaptureId;

pub const PAYLOAD_BITS: usize = 100;
pub const DATA_BITS: usize = 61;
pub const ECC_BITS: usize = PARITY_BITS as usize;
pub const VERSION_BITS: usize = 4;
pub const MAX_CORRECTABLE: u8 = T as u8;
pub const ERASURE_BUDGET: u8 = 2 * T as u8;
pub const DEFAULT_DEAD_ZONE: f32 = 0.25;

const VERSION_BCH5: u128 = 0b0001;
const SCHEMA_MASK: u128 = 0b11;
const CODE_MASK: u128 = (1 << (DATA_BITS + ECC_BITS)) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub capture_id: CaptureId,
    pub errors: u8,
    pub erasures: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardDecision {
    pub bits: [bool; PAYLOAD_BITS],
    pub erased: [bool; PAYLOAD_BITS],
}

pub fn encode(capture_id: CaptureId) -> [bool; PAYLOAD_BITS] {
    unpack(packet(capture_id))
}

pub fn decode(bits: &[bool; PAYLOAD_BITS], erased: &[bool; PAYLOAD_BITS]) -> Option<Decoded> {
    decode_packed(pack(bits), pack(erased))
}

pub fn soft_decide(logits: &[f32; PAYLOAD_BITS], dead_zone: f32) -> HardDecision {
    HardDecision {
        bits: core::array::from_fn(|i| logits[i] > 0.0),
        erased: core::array::from_fn(|i| is_erased(logits[i], dead_zone)),
    }
}

pub fn decode_logits(logits: &[f32; PAYLOAD_BITS], dead_zone: f32) -> Option<Decoded> {
    let (bits, erased) = logits.iter().fold((0u128, 0u128), |(bits, erased), &l| {
        (
            bits << 1 | (l > 0.0) as u128,
            erased << 1 | is_erased(l, dead_zone) as u128,
        )
    });
    decode_packed(bits, erased)
}

pub fn payload_tensor(bits: &[bool; PAYLOAD_BITS]) -> [f32; PAYLOAD_BITS] {
    core::array::from_fn(|i| if bits[i] { 1.0 } else { 0.0 })
}

fn is_erased(logit: f32, dead_zone: f32) -> bool {
    logit.abs().partial_cmp(&dead_zone) != Some(core::cmp::Ordering::Greater)
}

pub(crate) fn packet(capture_id: CaptureId) -> u128 {
    let data = capture_id.to_u64();
    let ecc = bch::parity(data << PAD_BITS);
    (data as u128) << (ECC_BITS + VERSION_BITS) | (ecc as u128) << VERSION_BITS | VERSION_BCH5
}

fn pack(bits: &[bool; PAYLOAD_BITS]) -> u128 {
    bits.iter().fold(0u128, |acc, &b| acc << 1 | b as u128)
}

fn unpack(word: u128) -> [bool; PAYLOAD_BITS] {
    core::array::from_fn(|i| word >> (PAYLOAD_BITS - 1 - i) & 1 == 1)
}

fn decode_packed(word: u128, erased: u128) -> Option<Decoded> {
    if (word ^ VERSION_BCH5) & SCHEMA_MASK & !erased != 0 {
        return None;
    }
    let received = word >> VERSION_BITS & CODE_MASK;
    let erased = erased >> VERSION_BITS & CODE_MASK;
    let erasures = erased.count_ones() as u8;
    if erasures > ERASURE_BUDGET {
        return None;
    }
    let fills = [received & !erased, received | erased];
    fills[..if erasures == 0 { 1 } else { 2 }]
        .iter()
        .find_map(|&fill| {
            let codeword = correct(fill)?;
            let errors = ((codeword ^ received) & !erased).count_ones() as u8;
            (2 * errors + erasures <= ERASURE_BUDGET).then(|| Decoded {
                capture_id: CaptureId::from_u64((codeword >> ECC_BITS) as u64)
                    .expect("61-bit data always fits a capture id"),
                errors,
                erasures,
            })
        })
}

fn correct(word: u128) -> Option<u128> {
    let data = (word >> ECC_BITS) as u64;
    let (message, ecc, _) = bch::correct(data << PAD_BITS, word as u64 & PARITY_MASK)?;
    Some(((message >> PAD_BITS) as u128) << ECC_BITS | ecc as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Rng;

    fn bits_of(s: &str) -> [bool; PAYLOAD_BITS] {
        assert_eq!(s.len(), PAYLOAD_BITS);
        core::array::from_fn(|i| s.as_bytes()[i] == b'1')
    }

    fn string_of(bits: &[bool]) -> String {
        bits.iter().map(|&b| if b { '1' } else { '0' }).collect()
    }

    fn id(v: u64) -> CaptureId {
        CaptureId::from_u64(v).unwrap()
    }

    fn random_id(rng: &mut Rng) -> CaptureId {
        id(rng.next() & CaptureId::MAX)
    }

    const NONE: [bool; PAYLOAD_BITS] = [false; PAYLOAD_BITS];

    const PYTHON_GOLDEN: [(u64, &str); 6] = [
        (
            0x0,
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
        ),
        (
            0x1,
            "0000000000000000000000000000000000000000000000000000000000001010010011110010000111011000100111010001",
        ),
        (
            0x1fff_ffff_ffff_ffff,
            "1111111111111111111111111111111111111111111111111111111111111111101010101001101001110000100100100001",
        ),
        (
            0x0123_4567_89ab_cdef,
            "0000100100011010001010110011110001001101010111100110111101111001000111011011000010110111111100000001",
        ),
        (
            0x01de_adbe_efca_fe42,
            "0000111011110101011011011111011101111110010101111111001000010011001010101111000011100110100111010001",
        ),
        (
            0x1f00_ff00_ff00_ff00,
            "1111100000000111111110000000011111111000000001111111100000000010000111110001010101010000101111010001",
        ),
    ];

    const RUST_REFERENCE_PACKET: &str = "1011011110011000111111000000011111011111011100000110110110111000110010101101111010011011000010000001";
    const RUST_REFERENCE_DATA: &str =
        "1011011110011000111111000000011111011111011100000110110110111";
    const RUST_REFERENCE_PADDED: &str = "1011010010100000000000000000000000000000000000000000000000000111110111111010101111110110000101010001";
    const RUST_REFERENCE_NONCANONICAL: &str = "1001110101000101100000011100111101010010000001100101100010001001111100010010000001101111101011000001";

    fn data_value(s: &str) -> u64 {
        u64::from_str_radix(s, 2).unwrap()
    }

    #[test]
    fn encode_matches_trustmark_python_golden_vectors() {
        for (value, expected) in PYTHON_GOLDEN {
            assert_eq!(string_of(&encode(id(value))), expected, "{value:#x}");
        }
    }

    #[test]
    fn encode_matches_trustmark_rust_reference_packets() {
        let encoded = encode(id(data_value(RUST_REFERENCE_DATA)));
        assert_eq!(string_of(&encoded), RUST_REFERENCE_PACKET);
        let padded = data_value(&RUST_REFERENCE_PADDED[..DATA_BITS]);
        assert_eq!(string_of(&encode(id(padded))), RUST_REFERENCE_PADDED);
    }

    #[test]
    fn golden_vectors_decode_cleanly() {
        for (value, packet) in PYTHON_GOLDEN {
            let decoded = decode(&bits_of(packet), &NONE).unwrap();
            assert_eq!(
                decoded,
                Decoded {
                    capture_id: id(value),
                    errors: 0,
                    erasures: 0
                }
            );
        }
    }

    #[test]
    fn rust_reference_single_flip_and_corrupted_version_prefix_decode() {
        let mut bits = bits_of(RUST_REFERENCE_PACKET);
        bits[0] = !bits[0];
        let decoded = decode(&bits, &NONE).unwrap();
        assert_eq!(decoded.capture_id.to_u64(), data_value(RUST_REFERENCE_DATA));
        assert_eq!(decoded.errors, 1);
        bits[96] = !bits[96];
        bits[97] = !bits[97];
        assert_eq!(
            decode(&bits, &NONE).unwrap().capture_id.to_u64(),
            data_value(RUST_REFERENCE_DATA)
        );
    }

    #[test]
    fn rust_reference_noncanonical_packet_is_rejected() {
        assert_eq!(decode(&bits_of(RUST_REFERENCE_NONCANONICAL), &NONE), None);
    }

    #[test]
    fn rust_reference_fully_corrupted_is_rejected() {
        let s = "0000000000000000000000000000000000000000000100000110110110111000110010101101111010011011000010000001";
        assert_eq!(decode(&bits_of(s), &NONE), None);
        assert_eq!(decode(&[true; PAYLOAD_BITS], &NONE), None);
    }

    #[test]
    fn layout_puts_capture_id_msb_first_then_ecc_then_version() {
        let cid = CaptureId::new(0x00AB_CDEF_0123_4567, 0b10110).unwrap();
        let bits = encode(cid);
        let data = bits[..DATA_BITS]
            .iter()
            .fold(0u64, |acc, &b| acc << 1 | b as u64);
        assert_eq!(data, cid.to_u64());
        let schema = bits[..5].iter().fold(0u8, |acc, &b| acc << 1 | b as u8);
        assert_eq!(schema, 0b10110);
        assert_eq!(&bits[96..], &[false, false, false, true]);
        let ecc = bits[DATA_BITS..96]
            .iter()
            .fold(0u64, |acc, &b| acc << 1 | b as u64);
        assert_eq!(ecc, bch::parity(cid.to_u64() << 3));
    }

    #[test]
    fn round_trip_random_ids() {
        let mut rng = Rng::new(10);
        for _ in 0..3000 {
            let cid = random_id(&mut rng);
            assert_eq!(
                decode(&encode(cid), &NONE),
                Some(Decoded {
                    capture_id: cid,
                    errors: 0,
                    erasures: 0
                })
            );
        }
    }

    #[test]
    fn up_to_five_errors_are_recovered_with_count() {
        let mut rng = Rng::new(11);
        for trial in 0..3000 {
            let cid = random_id(&mut rng);
            let mut bits = encode(cid);
            let count = trial % 6;
            let mut flipped = [false; 96];
            let mut n = 0;
            while n < count {
                let i = rng.below(96) as usize;
                if !flipped[i] {
                    flipped[i] = true;
                    bits[i] = !bits[i];
                    n += 1;
                }
            }
            let decoded = decode(&bits, &NONE).unwrap();
            assert_eq!(decoded.capture_id, cid);
            assert_eq!(decoded.errors as usize, count);
        }
    }

    #[test]
    fn six_plus_errors_are_rejected_or_flagged_as_different() {
        let mut rng = Rng::new(12);
        let mut rejected = 0;
        for trial in 0..2000 {
            let cid = random_id(&mut rng);
            let mut bits = encode(cid);
            let count = 6 + trial % 20;
            let mut flipped = [false; 96];
            let mut n = 0;
            while n < count {
                let i = rng.below(96) as usize;
                if !flipped[i] {
                    flipped[i] = true;
                    bits[i] = !bits[i];
                    n += 1;
                }
            }
            match decode(&bits, &NONE) {
                None => rejected += 1,
                Some(d) => {
                    assert_ne!(d.capture_id, cid);
                    assert!(d.errors <= MAX_CORRECTABLE);
                }
            }
        }
        assert!(rejected > 1900, "{rejected}");
    }

    #[test]
    fn schema_bits_must_say_bch5() {
        let mut bits = encode(id(42));
        bits[98] = true;
        assert_eq!(decode(&bits, &NONE), None);
        let mut bits = encode(id(42));
        bits[99] = false;
        assert_eq!(decode(&bits, &NONE), None);
        let mut erased = NONE;
        erased[98] = true;
        erased[99] = true;
        let mut bits = encode(id(42));
        bits[98] = true;
        bits[99] = false;
        assert_eq!(decode(&bits, &erased).unwrap().capture_id, id(42));
    }

    #[test]
    fn version_prefix_bits_are_ignored_like_python() {
        let mut bits = encode(id(7));
        bits[96] = true;
        bits[97] = true;
        assert_eq!(
            decode(&bits, &NONE).unwrap(),
            Decoded {
                capture_id: id(7),
                errors: 0,
                erasures: 0
            }
        );
    }

    #[test]
    fn erasures_alone_up_to_ten_are_filled() {
        let mut rng = Rng::new(13);
        for trial in 0..1500 {
            let cid = random_id(&mut rng);
            let mut bits = encode(cid);
            let mut erased = NONE;
            let count = trial % 11;
            let mut n = 0;
            while n < count {
                let i = rng.below(96) as usize;
                if !erased[i] {
                    erased[i] = true;
                    bits[i] = rng.next() & 1 == 1;
                    n += 1;
                }
            }
            let decoded = decode(&bits, &erased).unwrap();
            assert_eq!(
                decoded,
                Decoded {
                    capture_id: cid,
                    errors: 0,
                    erasures: count as u8
                }
            );
        }
    }

    #[test]
    fn errors_and_erasures_within_budget_are_recovered() {
        let mut rng = Rng::new(14);
        for trial in 0..3000 {
            let cid = random_id(&mut rng);
            let mut bits = encode(cid);
            let mut erased = NONE;
            let errors = trial % 6;
            let erasures = rng.below((ERASURE_BUDGET as u64 - 2 * errors as u64) + 1) as usize;
            let mut touched = [false; 96];
            let mut placed = 0;
            while placed < errors + erasures {
                let i = rng.below(96) as usize;
                if touched[i] {
                    continue;
                }
                touched[i] = true;
                if placed < errors {
                    bits[i] = !bits[i];
                } else {
                    erased[i] = true;
                    bits[i] = rng.next() & 1 == 1;
                }
                placed += 1;
            }
            let decoded = decode(&bits, &erased).unwrap();
            assert_eq!(
                decoded,
                Decoded {
                    capture_id: cid,
                    errors: errors as u8,
                    erasures: erasures as u8
                }
            );
        }
    }

    #[test]
    fn beyond_erasure_budget_never_returns_the_original_with_wrong_counts() {
        let mut rng = Rng::new(15);
        for trial in 0..1000 {
            let cid = random_id(&mut rng);
            let mut bits = encode(cid);
            let mut erased = NONE;
            let errors = 3 + trial % 3;
            let erasures = ERASURE_BUDGET as usize - 2 * errors + 1 + trial % 2;
            let mut touched = [false; 96];
            let mut placed = 0;
            while placed < errors + erasures {
                let i = rng.below(96) as usize;
                if touched[i] {
                    continue;
                }
                touched[i] = true;
                if placed < errors {
                    bits[i] = !bits[i];
                } else {
                    erased[i] = true;
                }
                placed += 1;
            }
            if let Some(d) = decode(&bits, &erased) {
                assert!(2 * d.errors + d.erasures <= ERASURE_BUDGET);
                assert_ne!(d.capture_id, cid);
            }
        }
    }

    #[test]
    fn more_than_ten_erasures_is_rejected() {
        let bits = encode(id(99));
        let mut erased = NONE;
        erased[..11].iter_mut().for_each(|e| *e = true);
        assert_eq!(decode(&bits, &erased), None);
        assert_eq!(decode(&bits, &[true; PAYLOAD_BITS]), None);
    }

    #[test]
    fn erased_version_bits_do_not_count_toward_budget() {
        let bits = encode(id(5));
        let mut erased = NONE;
        erased[96..].iter_mut().for_each(|e| *e = true);
        erased[..10].iter_mut().for_each(|e| *e = true);
        assert_eq!(decode(&bits, &erased).unwrap().erasures, 10);
    }

    #[test]
    fn soft_decide_applies_symmetric_dead_zone() {
        let mut logits = [3.0f32; PAYLOAD_BITS];
        logits[0] = -3.0;
        logits[1] = 0.2;
        logits[2] = -0.2;
        logits[3] = 0.25;
        logits[4] = -0.2500001;
        logits[5] = 0.0;
        logits[6] = f32::NAN;
        logits[7] = f32::INFINITY;
        logits[8] = f32::NEG_INFINITY;
        let d = soft_decide(&logits, 0.25);
        assert_eq!(
            &d.bits[..9],
            &[false, true, false, true, false, false, false, true, false]
        );
        assert_eq!(
            &d.erased[..9],
            &[false, true, true, true, false, true, true, false, false]
        );
        assert!(d.erased[9..].iter().all(|&e| !e));
    }

    #[test]
    fn soft_decide_is_antisymmetric() {
        let mut rng = Rng::new(16);
        for _ in 0..200 {
            let logits: [f32; PAYLOAD_BITS] = core::array::from_fn(|_| rng.unit() * 4.0 - 2.0);
            let negated = logits.map(|l| -l);
            let (a, b) = (soft_decide(&logits, 0.3), soft_decide(&negated, 0.3));
            assert_eq!(a.erased, b.erased);
            for i in 0..PAYLOAD_BITS {
                if !a.erased[i] {
                    assert_ne!(a.bits[i], b.bits[i]);
                }
            }
        }
    }

    #[test]
    fn nan_or_negative_dead_zone_behaves() {
        let logits = [1.0f32; PAYLOAD_BITS];
        assert!(soft_decide(&logits, f32::NAN).erased.iter().all(|&e| e));
        assert!(soft_decide(&logits, -1.0).erased.iter().all(|&e| !e));
        assert!(
            soft_decide(&[0.0; PAYLOAD_BITS], -1.0)
                .erased
                .iter()
                .all(|&e| !e)
        );
    }

    #[test]
    fn decode_logits_matches_soft_decide_then_decode() {
        let mut rng = Rng::new(17);
        for trial in 0..1000 {
            let cid = random_id(&mut rng);
            let bits = encode(cid);
            let logits: [f32; PAYLOAD_BITS] = core::array::from_fn(|i| {
                let magnitude = if rng.below(10) == 0 {
                    rng.unit() * 0.4
                } else {
                    1.0 + rng.unit() * 8.0
                };
                let sign = if bits[i] ^ (rng.below(40) == 0) {
                    1.0
                } else {
                    -1.0
                };
                sign * magnitude
            });
            let hard = soft_decide(&logits, DEFAULT_DEAD_ZONE);
            assert_eq!(
                decode_logits(&logits, DEFAULT_DEAD_ZONE),
                decode(&hard.bits, &hard.erased),
                "trial {trial}"
            );
        }
    }

    #[test]
    fn strong_logits_decode_to_id() {
        let cid = id(0x0abc_def0_1234_5678);
        let logits = encode(cid).map(|b| if b { 9.5 } else { -7.25 });
        assert_eq!(
            decode_logits(&logits, DEFAULT_DEAD_ZONE)
                .unwrap()
                .capture_id,
            cid
        );
    }

    #[test]
    fn noise_logits_rarely_decode() {
        let mut rng = Rng::new(18);
        let decoded = (0..5000)
            .filter(|_| {
                let logits: [f32; PAYLOAD_BITS] = core::array::from_fn(|_| rng.unit() * 10.0 - 5.0);
                decode_logits(&logits, DEFAULT_DEAD_ZONE).is_some()
            })
            .count();
        assert!(decoded < 20, "{decoded}");
    }

    #[test]
    fn payload_tensor_is_zero_one() {
        let bits = encode(id(0x1f00_ff00_ff00_ff00));
        let t = payload_tensor(&bits);
        for i in 0..PAYLOAD_BITS {
            assert_eq!(t[i], if bits[i] { 1.0 } else { 0.0 });
        }
    }

    #[test]
    fn pack_unpack_round_trip() {
        let mut rng = Rng::new(19);
        for _ in 0..500 {
            let w = (rng.next() as u128) << 64 | rng.next() as u128;
            let w = w & ((1u128 << PAYLOAD_BITS) - 1);
            assert_eq!(pack(&unpack(w)), w);
        }
    }

    #[test]
    fn decode_is_deterministic() {
        let mut rng = Rng::new(20);
        for _ in 0..300 {
            let bits: [bool; PAYLOAD_BITS] = core::array::from_fn(|_| rng.next() & 1 == 1);
            let erased: [bool; PAYLOAD_BITS] = core::array::from_fn(|_| rng.below(30) == 0);
            assert_eq!(decode(&bits, &erased), decode(&bits, &erased));
        }
    }
}
