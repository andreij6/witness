use crate::Error;
use crate::der::{self, Reader, Tag};
use crate::oid;
use p256::ecdsa::signature::hazmat::PrehashVerifier;
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256, Sha384, Sha512};

pub const MIN_RSA_BITS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigAlg {
    RsaSha256,
    RsaSha384,
    RsaSha512,
    EcdsaSha256,
    EcdsaSha384,
    EcdsaSha512,
}

impl SigAlg {
    pub fn from_algorithm_identifier(value: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(value);
        let id = r.expect(Tag::OID)?;
        let alg = match id {
            oid::SHA256_WITH_RSA => Self::RsaSha256,
            oid::SHA384_WITH_RSA => Self::RsaSha384,
            oid::SHA512_WITH_RSA => Self::RsaSha512,
            oid::ECDSA_SHA256 => Self::EcdsaSha256,
            oid::ECDSA_SHA384 => Self::EcdsaSha384,
            oid::ECDSA_SHA512 => Self::EcdsaSha512,
            _ => return Err(Error::UnsupportedAlgorithm),
        };
        if alg.is_rsa() && r.optional(Tag::NULL)?.is_some_and(|v| !v.is_empty()) {
            return Err(Error::Der);
        }
        r.finish()?;
        Ok(alg)
    }

    pub const fn is_rsa(self) -> bool {
        matches!(self, Self::RsaSha256 | Self::RsaSha384 | Self::RsaSha512)
    }

    fn digest(self, msg: &[u8], out: &mut [u8; 64]) -> usize {
        match self {
            Self::RsaSha256 | Self::EcdsaSha256 => {
                out[..32].copy_from_slice(&Sha256::digest(msg));
                32
            }
            Self::RsaSha384 | Self::EcdsaSha384 => {
                out[..48].copy_from_slice(&Sha384::digest(msg));
                48
            }
            Self::RsaSha512 | Self::EcdsaSha512 => {
                out.copy_from_slice(&Sha512::digest(msg));
                64
            }
        }
    }

    fn pkcs1v15(self) -> Pkcs1v15Sign {
        match self {
            Self::RsaSha384 | Self::EcdsaSha384 => Pkcs1v15Sign::new::<Sha384>(),
            Self::RsaSha512 | Self::EcdsaSha512 => Pkcs1v15Sign::new::<Sha512>(),
            Self::RsaSha256 | Self::EcdsaSha256 => Pkcs1v15Sign::new::<Sha256>(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicKey {
    P256(p256::ecdsa::VerifyingKey),
    P384(p384::ecdsa::VerifyingKey),
    Rsa(RsaPublicKey),
}

impl PublicKey {
    pub fn from_spki(spki_tlv: &[u8]) -> Result<Self, Error> {
        let mut outer = Reader::new(der::single(spki_tlv, Tag::SEQUENCE)?);
        let mut alg = Reader::new(outer.expect(Tag::SEQUENCE)?);
        let bits = der::octet_aligned_bits(outer.expect(Tag::BIT_STRING)?)?;
        outer.finish()?;
        match alg.expect(Tag::OID)? {
            oid::EC_PUBLIC_KEY => {
                let curve = alg.expect(Tag::OID)?;
                alg.finish()?;
                match curve {
                    oid::PRIME256V1 => p256::ecdsa::VerifyingKey::from_sec1_bytes(bits)
                        .map(Self::P256)
                        .map_err(|_| Error::Key),
                    oid::SECP384R1 => p384::ecdsa::VerifyingKey::from_sec1_bytes(bits)
                        .map(Self::P384)
                        .map_err(|_| Error::Key),
                    _ => Err(Error::UnsupportedAlgorithm),
                }
            }
            oid::RSA_ENCRYPTION => {
                if alg.expect(Tag::NULL)?.is_empty() {
                    alg.finish()?;
                } else {
                    return Err(Error::Der);
                }
                let mut key = Reader::new(der::single(bits, Tag::SEQUENCE)?);
                let n = der::unsigned_magnitude(key.expect(Tag::INTEGER)?)?;
                let e = der::unsigned_magnitude(key.expect(Tag::INTEGER)?)?;
                key.finish()?;
                let k = RsaPublicKey::new(BigUint::from_bytes_be(n), BigUint::from_bytes_be(e))
                    .map_err(|_| Error::Key)?;
                if k.n().bits() < MIN_RSA_BITS {
                    return Err(Error::Key);
                }
                Ok(Self::Rsa(k))
            }
            _ => Err(Error::UnsupportedAlgorithm),
        }
    }

    pub const fn is_p256(&self) -> bool {
        matches!(self, Self::P256(_))
    }

    pub fn verify(&self, alg: SigAlg, msg: &[u8], sig: &[u8]) -> Result<(), Error> {
        let mut buf = [0u8; 64];
        let len = alg.digest(msg, &mut buf);
        let digest = &buf[..len];
        let ok = match (self, alg.is_rsa()) {
            (Self::Rsa(k), true) => k.verify(alg.pkcs1v15(), digest, sig).is_ok(),
            (Self::P256(k), false) => p256::ecdsa::Signature::from_der(sig)
                .is_ok_and(|s| k.verify_prehash(digest, &s).is_ok()),
            (Self::P384(k), false) => p384::ecdsa::Signature::from_der(sig)
                .is_ok_and(|s| k.verify_prehash(digest, &s).is_ok()),
            _ => return Err(Error::AlgorithmMismatch),
        };
        if ok { Ok(()) } else { Err(Error::Signature) }
    }
}

#[cfg(test)]
mod tests;
