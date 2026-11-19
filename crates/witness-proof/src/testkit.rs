use crate::certificate::{DER_PREFIX, DOMAIN};
use crate::proof::{ROOTS_LEN, leaf_data};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use ic_bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use ic_bls12_381::{G1Affine, G1Projective, G2Affine, Scalar};
use ic_certification::certificate::{Certificate, Delegation};
use ic_certification::{HashTree, fork, label, leaf};
use serde_bytes::ByteBuf;
use witness_merkle::{Hash, Mmr, Smt, certified_data};

const BLS_DST: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

pub fn leb128(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

pub struct TestSubnet {
    sk: Scalar,
}

impl TestSubnet {
    pub fn new(seed: u64) -> Self {
        Self {
            sk: Scalar::from(seed.wrapping_mul(0x9E37_79B9).wrapping_add(17)),
        }
    }

    pub fn public_der(&self) -> Vec<u8> {
        let pk = G2Affine::from(G2Affine::generator() * self.sk);
        let mut der = DER_PREFIX.to_vec();
        der.extend_from_slice(&pk.to_compressed());
        der
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        let h =
            <G1Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(msg, BLS_DST);
        G1Affine::from(h * self.sk).to_compressed().to_vec()
    }

    pub fn certificate(&self, tree: HashTree, delegation: Option<Delegation<Vec<u8>>>) -> Vec<u8> {
        let mut msg = DOMAIN.to_vec();
        msg.extend_from_slice(&tree.digest());
        let cert = Certificate {
            signature: self.sign(&msg),
            tree,
            delegation,
        };
        serde_cbor::to_vec(&cert).expect("certificate cbor")
    }
}

pub fn state_tree(canister: &[u8], certified_data: &[u8], time_ns: u64) -> HashTree {
    fork(
        label(
            "canister",
            label(
                canister.to_vec(),
                label("certified_data", leaf(certified_data.to_vec())),
            ),
        ),
        label("time", leaf(leb128(time_ns))),
    )
}

pub fn ranges_cbor(ranges: &[(&[u8], &[u8])]) -> Vec<u8> {
    let r: Vec<(ByteBuf, ByteBuf)> = ranges
        .iter()
        .map(|(lo, hi)| (ByteBuf::from(lo.to_vec()), ByteBuf::from(hi.to_vec())))
        .collect();
    serde_cbor::to_vec(&r).expect("ranges cbor")
}

pub fn delegation_tree(subnet_id: &[u8], ranges: &[u8], key_der: &[u8], time_ns: u64) -> HashTree {
    fork(
        label(
            "subnet",
            label(
                subnet_id.to_vec(),
                fork(
                    label("canister_ranges", leaf(ranges.to_vec())),
                    label("public_key", leaf(key_der.to_vec())),
                ),
            ),
        ),
        label("time", leaf(leb128(time_ns))),
    )
}

pub fn delegated_certificate(
    root: &TestSubnet,
    subnet: &TestSubnet,
    subnet_id: &[u8],
    ranges: &[(&[u8], &[u8])],
    canister: &[u8],
    certified_data: &[u8],
    time_ns: u64,
) -> Vec<u8> {
    let inner = root.certificate(
        delegation_tree(
            subnet_id,
            &ranges_cbor(ranges),
            &subnet.public_der(),
            time_ns,
        ),
        None,
    );
    subnet.certificate(
        state_tree(canister, certified_data, time_ns),
        Some(Delegation {
            subnet_id: subnet_id.to_vec(),
            certificate: inner,
        }),
    )
}

pub struct TestLedger {
    pub subnet: TestSubnet,
    pub canister: Vec<u8>,
    pub time_ns: u64,
    mmr: Mmr<Vec<Hash>>,
    smt: Smt<BTreeMap<(u8, u128), Hash>>,
}

impl TestLedger {
    pub fn new(subnet: TestSubnet, canister: &[u8], time_ns: u64) -> Self {
        Self {
            subnet,
            canister: canister.to_vec(),
            time_ns,
            mmr: Mmr::new(Vec::new()).expect("mmr"),
            smt: Smt::new(BTreeMap::new()),
        }
    }

    pub fn append(
        &mut self,
        claim: &[u8],
        c2pa_claim: &[u8],
        cose: &[u8],
        anchored_ns: u64,
    ) -> u64 {
        self.mmr
            .append(&leaf_data(claim, c2pa_claim, cose, anchored_ns))
            .expect("append")
    }

    pub fn insert(&mut self, key: u128, value: &Hash) {
        self.smt.insert(key, value);
    }

    pub fn leaf_count(&self) -> u64 {
        self.mmr.leaf_count()
    }

    pub fn mmr_root(&self) -> Hash {
        self.mmr.root().expect("mmr root")
    }

    pub fn certified_data(&self) -> Hash {
        certified_data(&self.mmr_root(), self.leaf_count(), &self.smt.root())
    }

    pub fn certificate(&self) -> Vec<u8> {
        self.subnet.certificate(
            state_tree(&self.canister, &self.certified_data(), self.time_ns),
            None,
        )
    }

    pub fn record_proof(&self, ix: u64) -> Vec<ByteBuf> {
        let mut path: Vec<ByteBuf> = self
            .mmr
            .prove(ix)
            .expect("prove")
            .path
            .iter()
            .map(|h| ByteBuf::from(h.to_vec()))
            .collect();
        path.push(ByteBuf::from(self.smt.root().to_vec()));
        path
    }

    pub fn smt_bundle(&self, key: u128) -> Vec<u8> {
        let mut b = self.smt.prove(key).to_bytes();
        b.reserve(ROOTS_LEN);
        b.extend_from_slice(&self.mmr_root());
        b.extend_from_slice(&self.leaf_count().to_be_bytes());
        b
    }

    pub fn shards_bundle(&self, key: u128) -> Vec<u8> {
        let cert = self.certificate();
        let mut b = u32::try_from(cert.len())
            .expect("cert len")
            .to_be_bytes()
            .to_vec();
        b.extend_from_slice(&cert);
        b.extend_from_slice(&self.smt_bundle(key));
        b
    }
}
