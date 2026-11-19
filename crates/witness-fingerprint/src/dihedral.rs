use crate::dct::{Coefficients, PDQ_FREQS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Dihedral {
    Identity,
    Rotate90,
    Rotate180,
    Rotate270,
    MirrorHorizontal,
    MirrorVertical,
    Transpose,
    AntiTranspose,
}

const fn odd_frequency_sign(index: usize) -> i64 {
    if index & 1 == 0 { -1 } else { 1 }
}

impl Dihedral {
    pub const ALL: [Self; 8] = [
        Self::Identity,
        Self::Rotate90,
        Self::Rotate180,
        Self::Rotate270,
        Self::MirrorHorizontal,
        Self::MirrorVertical,
        Self::Transpose,
        Self::AntiTranspose,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    const fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::Rotate90 | Self::Rotate270 | Self::Transpose | Self::AntiTranspose
        )
    }

    const fn negates(self, row: usize, column: usize) -> bool {
        let (vertical, horizontal) = match self {
            Self::Identity | Self::Transpose => (false, false),
            Self::Rotate90 | Self::MirrorHorizontal => (false, true),
            Self::Rotate270 | Self::MirrorVertical => (true, false),
            Self::Rotate180 | Self::AntiTranspose => (true, true),
        };
        let mut sign = 1;
        if vertical {
            sign *= odd_frequency_sign(row);
        }
        if horizontal {
            sign *= odd_frequency_sign(column);
        }
        sign < 0
    }

    pub fn apply(self, source: &Coefficients) -> Coefficients {
        let mut out = [[0i64; PDQ_FREQS]; PDQ_FREQS];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                let original = if self.swaps_axes() {
                    source[j][i]
                } else {
                    source[i][j]
                };
                *value = if self.negates(i, j) {
                    -original
                } else {
                    original
                };
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;

    fn random(rng: &mut Rng) -> Coefficients {
        let mut c = [[0i64; PDQ_FREQS]; PDQ_FREQS];
        c.iter_mut()
            .flatten()
            .for_each(|v| *v = rng.next() as i32 as i64);
        c
    }

    fn compose(a: Dihedral, b: Dihedral, c: &Coefficients) -> Coefficients {
        b.apply(&a.apply(c))
    }

    #[test]
    fn identity_is_noop() {
        let mut rng = Rng::new(5);
        let c = random(&mut rng);
        assert_eq!(Dihedral::Identity.apply(&c), c);
    }

    #[test]
    fn involutions_square_to_identity() {
        let mut rng = Rng::new(6);
        let c = random(&mut rng);
        for d in [
            Dihedral::Rotate180,
            Dihedral::MirrorHorizontal,
            Dihedral::MirrorVertical,
            Dihedral::Transpose,
            Dihedral::AntiTranspose,
        ] {
            assert_eq!(compose(d, d, &c), c, "{d:?}");
        }
    }

    #[test]
    fn group_relations_hold() {
        let mut rng = Rng::new(7);
        let c = random(&mut rng);
        let r90 = Dihedral::Rotate90;
        assert_eq!(compose(r90, r90, &c), Dihedral::Rotate180.apply(&c));
        assert_eq!(
            compose(Dihedral::Rotate180, r90, &c),
            Dihedral::Rotate270.apply(&c)
        );
        assert_eq!(compose(r90, Dihedral::Rotate270, &c), c);
        assert_eq!(
            compose(Dihedral::MirrorHorizontal, Dihedral::MirrorVertical, &c),
            Dihedral::Rotate180.apply(&c)
        );
        assert_eq!(
            compose(Dihedral::Transpose, Dihedral::Rotate180, &c),
            Dihedral::AntiTranspose.apply(&c)
        );
    }

    #[test]
    fn all_eight_are_distinct_on_generic_input() {
        let mut rng = Rng::new(8);
        let c = random(&mut rng);
        let images: Vec<Coefficients> = Dihedral::ALL.iter().map(|d| d.apply(&c)).collect();
        for i in 0..8 {
            for j in i + 1..8 {
                assert_ne!(images[i], images[j]);
            }
        }
        assert!(
            Dihedral::ALL
                .iter()
                .enumerate()
                .all(|(i, d)| d.index() == i)
        );
    }

    #[test]
    fn mirror_negates_odd_horizontal_frequencies_only() {
        let mut c = [[0i64; PDQ_FREQS]; PDQ_FREQS];
        c[3][0] = 10;
        c[3][1] = 20;
        let m = Dihedral::MirrorHorizontal.apply(&c);
        assert_eq!(m[3][0], -10);
        assert_eq!(m[3][1], 20);
    }
}
