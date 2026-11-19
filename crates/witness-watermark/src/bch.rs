pub(crate) const T: usize = 5;
pub(crate) const PARITY_BITS: u32 = 35;
pub(crate) const PAD_BITS: u32 = 3;
pub(crate) const PARITY_MASK: u64 = (1 << PARITY_BITS) - 1;

const N: usize = 127;
const FIELD_POLY: u16 = 137;
const MESSAGE_BITS: u32 = 64;
const CODE_BITS: u32 = MESSAGE_BITS + PARITY_BITS;
const PAD_MASK: u128 = ((1 << PAD_BITS) - 1) << PARITY_BITS;
const SYNDROMES: usize = 2 * T;
const LOCATOR_LEN: usize = SYNDROMES + 1;

struct Field {
    exp: [u8; 2 * N],
    log: [u8; N + 1],
}

const FIELD: Field = build_field();

const fn build_field() -> Field {
    let mut exp = [0u8; 2 * N];
    let mut log = [0u8; N + 1];
    let mut x: u16 = 1;
    let mut i = 0;
    while i < N {
        exp[i] = x as u8;
        exp[i + N] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x80 != 0 {
            x ^= FIELD_POLY;
        }
        i += 1;
    }
    Field { exp, log }
}

const fn mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        FIELD.exp[FIELD.log[a as usize] as usize + FIELD.log[b as usize] as usize]
    }
}

const fn div(a: u8, b: u8) -> u8 {
    if a == 0 {
        0
    } else {
        FIELD.exp[FIELD.log[a as usize] as usize + N - FIELD.log[b as usize] as usize]
    }
}

const fn alpha_pow(e: usize) -> u8 {
    FIELD.exp[e % N]
}

pub(crate) const GENERATOR: u64 = build_generator();

const fn build_generator() -> u64 {
    let mut g = [0u8; PARITY_BITS as usize + 1];
    g[0] = 1;
    let mut degree = 0;
    let mut odd = 1;
    while odd < SYNDROMES {
        let mut root = odd;
        let mut k = 0;
        while k < 7 {
            let a = alpha_pow(root);
            g[degree + 1] = g[degree];
            let mut i = degree;
            while i > 0 {
                g[i] = g[i - 1] ^ mul(g[i], a);
                i -= 1;
            }
            g[0] = mul(g[0], a);
            degree += 1;
            root = (root * 2) % N;
            k += 1;
        }
        odd += 2;
    }
    let mut packed = 0u64;
    let mut i = 0;
    while i <= degree {
        packed |= (g[i] as u64 & 1) << i;
        i += 1;
    }
    packed
}

const PARITY_TABLE: [u64; 256] = build_parity_table();

const fn build_parity_table() -> [u64; 256] {
    let feedback = GENERATOR & PARITY_MASK;
    let mut table = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        let mut r = (i as u64) << (PARITY_BITS - 8);
        let mut bit = 0;
        while bit < 8 {
            let carry = r >> (PARITY_BITS - 1) & 1;
            r = (r << 1) & PARITY_MASK;
            if carry == 1 {
                r ^= feedback;
            }
            bit += 1;
        }
        table[i] = r;
        i += 1;
    }
    table
}

pub(crate) fn parity(message: u64) -> u64 {
    message.to_be_bytes().iter().fold(0u64, |r, &byte| {
        let index = ((r >> (PARITY_BITS - 8)) as u8 ^ byte) as usize;
        ((r << 8) & PARITY_MASK) ^ PARITY_TABLE[index]
    })
}

fn syndromes(word: u128) -> [u8; SYNDROMES] {
    let mut s = [0u8; SYNDROMES];
    let mut rest = word;
    while rest != 0 {
        let degree = rest.trailing_zeros() as usize;
        rest &= rest - 1;
        for k in 0..T {
            s[2 * k] ^= alpha_pow((2 * k + 1) * degree);
        }
    }
    for k in 0..T {
        s[2 * k + 1] = mul(s[k], s[k]);
    }
    s
}

fn error_locator(s: &[u8; SYNDROMES]) -> Option<([u8; LOCATOR_LEN], usize)> {
    let mut c = [0u8; LOCATOR_LEN];
    let mut b = [0u8; LOCATOR_LEN];
    c[0] = 1;
    b[0] = 1;
    let (mut length, mut shift, mut last) = (0usize, 1usize, 1u8);
    for n in 0..SYNDROMES {
        let mut d = s[n];
        for i in 1..=length {
            d ^= mul(c[i], s[n - i]);
        }
        if d == 0 {
            shift += 1;
            continue;
        }
        let previous = c;
        let factor = div(d, last);
        for i in 0..LOCATOR_LEN.saturating_sub(shift) {
            c[i + shift] ^= mul(factor, b[i]);
        }
        if 2 * length <= n {
            length = n + 1 - length;
            b = previous;
            last = d;
            shift = 1;
        } else {
            shift += 1;
        }
    }
    (length <= T).then_some((c, length))
}

fn error_positions(locator: &[u8; LOCATOR_LEN], length: usize) -> Option<u128> {
    let mut mask = 0u128;
    let mut found = 0;
    for degree in 0..CODE_BITS as usize {
        let inverse = (N - degree % N) % N;
        let value = locator[..=length]
            .iter()
            .enumerate()
            .fold(0u8, |acc, (i, &coefficient)| {
                acc ^ mul(coefficient, alpha_pow(inverse * i))
            });
        if value == 0 {
            mask |= 1 << degree;
            found += 1;
        }
    }
    (found == length && mask & PAD_MASK == 0).then_some(mask)
}

pub(crate) fn correct(message: u64, received_parity: u64) -> Option<(u64, u64, u32)> {
    if parity(message) == received_parity {
        return Some((message, received_parity, 0));
    }
    let word = (message as u128) << PARITY_BITS | received_parity as u128;
    let (locator, length) = error_locator(&syndromes(word))?;
    let fixed = word ^ error_positions(&locator, length)?;
    let (message, fixed_parity) = ((fixed >> PARITY_BITS) as u64, fixed as u64 & PARITY_MASK);
    (parity(message) == fixed_parity).then_some((message, fixed_parity, length as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Rng;

    fn naive_parity(message: u64) -> u64 {
        let feedback = GENERATOR & PARITY_MASK;
        (0..64).rev().fold(0u64, |r, i| {
            let carry = (message >> i & 1) ^ (r >> (PARITY_BITS - 1) & 1);
            let r = (r << 1) & PARITY_MASK;
            if carry == 1 { r ^ feedback } else { r }
        })
    }

    fn poly_eval(poly: u128, x_log: usize) -> u8 {
        (0..128)
            .filter(|d| poly >> d & 1 == 1)
            .fold(0u8, |acc, d| acc ^ alpha_pow(x_log * d))
    }

    fn flip_random(rng: &mut Rng, count: usize, positions: &[u32]) -> u128 {
        let mut mask = 0u128;
        while (mask.count_ones() as usize) < count {
            mask |= 1 << positions[rng.below(positions.len() as u64) as usize];
        }
        mask
    }

    fn transmitted_degrees() -> Vec<u32> {
        (0..CODE_BITS).filter(|d| PAD_MASK >> d & 1 == 0).collect()
    }

    #[test]
    fn field_tables_are_inverse_bijections() {
        let mut seen = [false; 128];
        for i in 0..N {
            let x = FIELD.exp[i];
            assert_ne!(x, 0);
            assert!(!seen[x as usize]);
            seen[x as usize] = true;
            assert_eq!(FIELD.log[x as usize] as usize, i);
            assert_eq!(FIELD.exp[i + N], x);
        }
        assert_eq!(alpha_pow(N), 1);
        assert_eq!(alpha_pow(7), (FIELD_POLY & 0x7f) as u8);
    }

    #[test]
    fn field_arithmetic_laws() {
        for a in 0..=127u8 {
            assert_eq!(mul(a, 1), a);
            assert_eq!(mul(a, 0), 0);
            assert_eq!(div(a, 1), a);
            for b in 1..=127u8 {
                assert_eq!(mul(a, b), mul(b, a));
                assert_eq!(div(mul(a, b), b), a);
            }
        }
    }

    #[test]
    fn generator_has_degree_35_and_vanishes_on_designed_roots() {
        assert_eq!(64 - GENERATOR.leading_zeros(), PARITY_BITS + 1);
        assert_eq!(GENERATOR & 1, 1);
        for j in 1..=SYNDROMES {
            assert_eq!(poly_eval(GENERATOR as u128, j), 0, "root alpha^{j}");
        }
        assert_eq!(GENERATOR, 0xC_A760_24D7);
    }

    #[test]
    fn generator_divides_x127_minus_1() {
        let mut r: u128 = 1 << N | 1;
        let g = GENERATOR as u128;
        while 128 - r.leading_zeros() > PARITY_BITS + 1 {
            let shift = 128 - r.leading_zeros() - (PARITY_BITS + 1);
            r ^= g << shift;
        }
        if r >> PARITY_BITS & 1 == 1 {
            r ^= g;
        }
        assert_eq!(r, 0);
    }

    #[test]
    fn table_parity_matches_bitwise_lfsr() {
        let mut rng = Rng::new(1);
        for _ in 0..2000 {
            let m = rng.next();
            assert_eq!(parity(m), naive_parity(m));
        }
        assert_eq!(parity(0), 0);
        assert_eq!(parity(u64::MAX), naive_parity(u64::MAX));
    }

    #[test]
    fn parity_is_linear() {
        let mut rng = Rng::new(2);
        for _ in 0..500 {
            let (a, b) = (rng.next(), rng.next());
            assert_eq!(parity(a ^ b), parity(a) ^ parity(b));
        }
    }

    #[test]
    fn codewords_have_zero_syndromes() {
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            let m = rng.next() & !((1 << PAD_BITS) - 1);
            let word = (m as u128) << PARITY_BITS | parity(m) as u128;
            assert_eq!(syndromes(word), [0; SYNDROMES]);
        }
    }

    #[test]
    fn corrects_up_to_five_errors_anywhere_transmitted() {
        let mut rng = Rng::new(4);
        let positions = transmitted_degrees();
        for trial in 0..3000 {
            let m = rng.next() & !((1 << PAD_BITS) - 1);
            let p = parity(m);
            let count = trial % (T + 1);
            let flips = flip_random(&mut rng, count, &positions);
            let word = ((m as u128) << PARITY_BITS | p as u128) ^ flips;
            let got = correct((word >> PARITY_BITS) as u64, word as u64 & PARITY_MASK);
            assert_eq!(got, Some((m, p, count as u32)), "trial {trial}");
        }
    }

    #[test]
    fn six_or_more_errors_never_return_the_original() {
        let mut rng = Rng::new(5);
        let positions = transmitted_degrees();
        let mut rejected = 0;
        for trial in 0..2000 {
            let m = rng.next() & !((1 << PAD_BITS) - 1);
            let p = parity(m);
            let count = T + 1 + trial % 12;
            let flips = flip_random(&mut rng, count, &positions);
            let word = ((m as u128) << PARITY_BITS | p as u128) ^ flips;
            match correct((word >> PARITY_BITS) as u64, word as u64 & PARITY_MASK) {
                None => rejected += 1,
                Some((got, got_parity, n)) => {
                    assert_ne!(got, m);
                    assert!(n as usize <= T);
                    assert_eq!(parity(got), got_parity);
                    assert_eq!(got & ((1 << PAD_BITS) - 1), 0);
                }
            }
        }
        assert!(rejected > 1800, "rejected {rejected}");
    }

    #[test]
    fn errors_on_pad_bits_are_rejected() {
        let mut rng = Rng::new(6);
        for _ in 0..300 {
            let m = rng.next() | 1;
            let p = parity(m);
            let transmitted = m & !((1 << PAD_BITS) - 1);
            assert_eq!(correct(transmitted, p), None);
        }
    }

    #[test]
    fn pad_error_plus_other_errors_is_rejected() {
        let mut rng = Rng::new(7);
        let positions = transmitted_degrees();
        for trial in 0..500 {
            let m = (rng.next() & !7) | 0b010;
            let p = parity(m);
            let flips = flip_random(&mut rng, trial % 5, &positions);
            let word = ((m as u128 & !0b111) << PARITY_BITS | p as u128) ^ flips;
            if let Some((got, _, _)) =
                correct((word >> PARITY_BITS) as u64, word as u64 & PARITY_MASK)
            {
                assert_ne!(got, m);
                assert_eq!(got & 0b111, 0);
            }
        }
    }

    #[test]
    fn all_ones_word_is_rejected() {
        assert_eq!(correct(!7, PARITY_MASK), None);
    }

    #[test]
    fn error_locator_reports_length_beyond_capacity_as_none() {
        let mut s = [0u8; SYNDROMES];
        let mut rng = Rng::new(8);
        let mut rejected = 0;
        for _ in 0..200 {
            s.iter_mut().for_each(|v| *v = (rng.below(127) + 1) as u8);
            if error_locator(&s).is_none() {
                rejected += 1;
            }
        }
        assert!(rejected > 0);
    }

    #[test]
    fn correct_is_deterministic() {
        let mut rng = Rng::new(9);
        for _ in 0..200 {
            let (m, p) = (rng.next(), rng.next() & PARITY_MASK);
            assert_eq!(correct(m, p), correct(m, p));
        }
    }
}
