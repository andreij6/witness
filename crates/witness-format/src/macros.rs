macro_rules! code_enum {
    ($(#[$m:meta])* $vis:vis enum $name:ident { $($var:ident = $code:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        $vis enum $name { $($var = $code),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$var),+];

            pub const fn code(self) -> u8 {
                self as u8
            }

            pub const fn from_code(code: u8) -> Option<Self> {
                match code {
                    $($code => Some(Self::$var),)+
                    _ => None,
                }
            }
        }

        impl $crate::cbor::Encode for $name {
            fn encode(&self, e: &mut $crate::cbor::Encoder<'_>) {
                e.u64(u64::from(self.code()));
            }
        }

        impl<'a> $crate::cbor::Decode<'a> for $name {
            fn decode(d: &mut $crate::cbor::Decoder<'a>) -> Result<Self, $crate::cbor::Error> {
                u8::try_from(d.u64()?)
                    .ok()
                    .and_then(Self::from_code)
                    .ok_or($crate::cbor::Error::Range)
            }
        }
    };
}

macro_rules! bitset {
    ($(#[$m:meta])* $vis:vis struct $name:ident: $t:ty { $(const $flag:ident = $bit:literal;)+ }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
        $vis struct $name($t);

        impl $name {
            $(pub const $flag: Self = Self(1 << $bit);)+
            pub const EMPTY: Self = Self(0);
            pub const ALL: Self = Self(0 $(| (1 << $bit))+);
            pub const FLAGS: &'static [Self] = &[$(Self::$flag),+];

            pub const fn bits(self) -> $t {
                self.0
            }

            pub const fn from_bits(bits: $t) -> Option<Self> {
                if bits & !Self::ALL.0 == 0 { Some(Self(bits)) } else { None }
            }

            pub const fn from_bits_truncate(bits: $t) -> Self {
                Self(bits & Self::ALL.0)
            }

            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            pub const fn intersects(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }

            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            pub const fn intersection(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }

            pub const fn difference(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }

            pub const fn len(self) -> u32 {
                self.0.count_ones()
            }

            pub fn insert(&mut self, other: Self) {
                self.0 |= other.0;
            }

            pub fn remove(&mut self, other: Self) {
                self.0 &= !other.0;
            }

            pub fn set(&mut self, other: Self, on: bool) {
                if on { self.insert(other) } else { self.remove(other) }
            }

            pub fn iter(self) -> impl Iterator<Item = Self> {
                Self::FLAGS.iter().copied().filter(move |f| self.contains(*f))
            }
        }

        impl core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                self.union(rhs)
            }
        }

        impl core::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) {
                self.insert(rhs);
            }
        }

        impl core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                self.intersection(rhs)
            }
        }

        impl $crate::cbor::Encode for $name {
            fn encode(&self, e: &mut $crate::cbor::Encoder<'_>) {
                e.u64(u64::from(self.0));
            }
        }

        impl<'a> $crate::cbor::Decode<'a> for $name {
            fn decode(d: &mut $crate::cbor::Decoder<'a>) -> Result<Self, $crate::cbor::Error> {
                <$t>::try_from(d.u64()?)
                    .ok()
                    .and_then(Self::from_bits)
                    .ok_or($crate::cbor::Error::Range)
            }
        }
    };
}
