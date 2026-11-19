use super::*;
use crate::testkit::{
    TestSubnet, delegated_certificate, delegation_tree, leb128 as encode_leb, ranges_cbor,
    state_tree,
};
use ic_certification::certificate::Delegation;
use ic_certification::{fork, label, leaf};

const CANISTER: [u8; 10] = [0, 0, 0, 0, 0, 0x10, 0, 1, 1, 1];
const SUBNET: [u8; 29] = [7; 29];
const NOW: u64 = 1_763_510_400_000_000_000;

fn lo() -> [u8; 10] {
    [0, 0, 0, 0, 0, 0x10, 0, 0, 1, 1]
}

fn hi() -> [u8; 10] {
    [0, 0, 0, 0, 0, 0x1f, 0xff, 0xff, 1, 1]
}

#[test]
fn raw_key_strips_the_bls_der_prefix() {
    let der = TestSubnet::new(1).public_der();
    assert_eq!(raw_key(&der).unwrap(), &der[DER_PREFIX.len()..]);
    assert_eq!(
        raw_key(&der[..der.len() - 1]),
        Err(Error::Certificate("root key"))
    );
    let mut wrong = der.clone();
    wrong[3] ^= 1;
    assert!(raw_key(&wrong).is_err());
    assert!(raw_key(&[]).is_err());
    let mut long = der;
    long.push(0);
    assert!(raw_key(&long).is_err());
}

#[test]
fn leb128_roundtrips_and_rejects_malformed() {
    for v in [0, 1, 127, 128, 300, NOW, u64::MAX] {
        assert_eq!(leb128(&encode_leb(v)), Ok(v));
    }
    assert!(leb128(&[]).is_err());
    assert!(leb128(&[0x80]).is_err());
    assert!(leb128(&[0x01, 0x00]).is_err());
    assert!(leb128(&[0xff; 10]).is_err());
    let mut over = vec![0xff; 9];
    over.push(0x02);
    assert!(leb128(&over).is_err());
    assert!(leb128(&[0x80; 11]).is_err());
}

#[test]
fn freshness_is_symmetric_and_optional() {
    assert!(fresh(0, NOW, 0).is_ok());
    assert!(fresh(NOW, NOW, 1).is_ok());
    assert!(fresh(NOW - 10, NOW, 10).is_ok());
    assert!(fresh(NOW + 10, NOW, 10).is_ok());
    assert_eq!(fresh(NOW - 11, NOW, 10), Err(Error::Certificate("stale")));
    assert!(fresh(NOW + 11, NOW, 10).is_err());
    assert!(fresh(0, u64::MAX, u64::MAX - 1).is_err());
}

#[test]
fn direct_certificate_yields_certified_data() {
    let root = TestSubnet::new(1);
    let cert = root.certificate(state_tree(&CANISTER, &[5; 32], NOW), None);
    let c = verify(
        &cert,
        &root.public_der(),
        &CANISTER,
        NOW,
        DEFAULT_MAX_AGE_NS,
    )
    .unwrap();
    assert_eq!(
        c,
        Certified {
            data: vec![5; 32],
            time_ns: NOW
        }
    );
    assert_eq!(
        verify(
            &cert,
            &root.public_der(),
            &CANISTER,
            NOW + DEFAULT_MAX_AGE_NS + 1,
            DEFAULT_MAX_AGE_NS
        ),
        Err(Error::Certificate("stale"))
    );
    assert!(verify(&cert, &root.public_der(), &CANISTER, 0, 0).is_ok());
}

#[test]
fn wrong_key_or_canister_is_rejected() {
    let root = TestSubnet::new(1);
    let cert = root.certificate(state_tree(&CANISTER, &[5; 32], NOW), None);
    assert_eq!(
        verify(&cert, &TestSubnet::new(2).public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("signature"))
    );
    assert_eq!(
        verify(&cert, &root.public_der(), &[9; 10], NOW, 0),
        Err(Error::Certificate("certified data"))
    );
    assert_eq!(
        verify(&cert, &[1, 2], &CANISTER, NOW, 0),
        Err(Error::Certificate("root key"))
    );
}

#[test]
fn tampered_trees_and_bytes_are_rejected() {
    let root = TestSubnet::new(1);
    let good = root.certificate(state_tree(&CANISTER, &[5; 32], NOW), None);
    let forged = TestSubnet::new(1).certificate(state_tree(&CANISTER, &[6; 32], NOW), None);
    let mut spliced: ic_certification::Certificate = serde_cbor::from_slice(&forged).unwrap();
    let original: ic_certification::Certificate = serde_cbor::from_slice(&good).unwrap();
    spliced.signature = original.signature;
    let spliced = serde_cbor::to_vec(&spliced).unwrap();
    assert_eq!(
        verify(&spliced, &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("signature"))
    );
    assert_eq!(
        verify(&[0xff, 0x00], &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("cbor"))
    );
    for cut in [0, 1, good.len() / 2, good.len() - 1] {
        assert!(verify(&good[..cut], &root.public_der(), &CANISTER, NOW, 0).is_err());
    }
    let mut flips = 0;
    for i in (0..good.len()).step_by(7) {
        let mut b = good.clone();
        b[i] ^= 0x04;
        if let Ok(c) = verify(&b, &root.public_der(), &CANISTER, NOW, 0) {
            assert_eq!(c.data, vec![5; 32]);
            flips += 1;
        }
    }
    assert!(flips < good.len() / 7);
}

#[test]
fn missing_time_is_rejected() {
    let root = TestSubnet::new(1);
    let tree = label(
        "canister",
        label(
            CANISTER.to_vec(),
            label("certified_data", leaf(vec![1u8; 32])),
        ),
    );
    let cert = root.certificate(tree, None);
    assert_eq!(
        verify(&cert, &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("time"))
    );
}

#[test]
fn delegated_certificate_verifies_inside_ranges() {
    let root = TestSubnet::new(1);
    let subnet = TestSubnet::new(2);
    let cert = delegated_certificate(
        &root,
        &subnet,
        &SUBNET,
        &[(&lo(), &hi())],
        &CANISTER,
        &[8; 32],
        NOW,
    );
    let c = verify(
        &cert,
        &root.public_der(),
        &CANISTER,
        NOW,
        DEFAULT_MAX_AGE_NS,
    )
    .unwrap();
    assert_eq!(c.data, vec![8; 32]);
    assert_eq!(
        verify(&cert, &subnet.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("signature"))
    );
}

#[test]
fn delegation_outside_ranges_is_rejected() {
    let root = TestSubnet::new(1);
    let subnet = TestSubnet::new(2);
    let other = [0u8, 0, 0, 0, 0, 0x30, 0, 1, 1, 1];
    let cert = delegated_certificate(
        &root,
        &subnet,
        &SUBNET,
        &[(&lo(), &hi())],
        &other,
        &[8; 32],
        NOW,
    );
    assert_eq!(
        verify(&cert, &root.public_der(), &other, NOW, 0),
        Err(Error::Certificate("canister outside delegation"))
    );
    let empty = delegated_certificate(&root, &subnet, &SUBNET, &[], &CANISTER, &[8; 32], NOW);
    assert!(verify(&empty, &root.public_der(), &CANISTER, NOW, 0).is_err());
}

#[test]
fn delegation_signed_by_another_root_is_rejected() {
    let root = TestSubnet::new(1);
    let subnet = TestSubnet::new(2);
    let cert = delegated_certificate(
        &TestSubnet::new(3),
        &subnet,
        &SUBNET,
        &[(&lo(), &hi())],
        &CANISTER,
        &[8; 32],
        NOW,
    );
    assert_eq!(
        verify(&cert, &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("signature"))
    );
}

#[test]
fn nested_delegation_is_rejected() {
    let root = TestSubnet::new(1);
    let subnet = TestSubnet::new(2);
    let inner = delegated_certificate(
        &root,
        &subnet,
        &SUBNET,
        &[(&lo(), &hi())],
        &CANISTER,
        &[8; 32],
        NOW,
    );
    let outer = subnet.certificate(
        state_tree(&CANISTER, &[8; 32], NOW),
        Some(Delegation {
            subnet_id: SUBNET.to_vec(),
            certificate: inner,
        }),
    );
    assert_eq!(
        verify(&outer, &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("nested delegation"))
    );
}

#[test]
fn delegation_without_subnet_key_is_rejected() {
    let root = TestSubnet::new(1);
    let subnet = TestSubnet::new(2);
    let tree = fork(
        label(
            "subnet",
            label(
                SUBNET.to_vec(),
                label("canister_ranges", leaf(ranges_cbor(&[(&lo(), &hi())]))),
            ),
        ),
        label("time", leaf(encode_leb(NOW))),
    );
    let cert = subnet.certificate(
        state_tree(&CANISTER, &[8; 32], NOW),
        Some(Delegation {
            subnet_id: SUBNET.to_vec(),
            certificate: root.certificate(tree, None),
        }),
    );
    assert_eq!(
        verify(&cert, &root.public_der(), &CANISTER, NOW, 0),
        Err(Error::Certificate("subnet key"))
    );
}

#[test]
fn sharded_canister_ranges_are_honoured() {
    let shard_a = [0u8, 0, 0, 0, 0, 0x00, 0, 0, 1, 1];
    let shard_b = lo();
    let tree = fork(
        label(
            "canister_ranges",
            label(
                SUBNET.to_vec(),
                fork(
                    label(
                        shard_a.to_vec(),
                        leaf(ranges_cbor(&[(
                            &shard_a,
                            &[0, 0, 0, 0, 0, 0x0f, 0xff, 0xff, 1, 1],
                        )])),
                    ),
                    label(shard_b.to_vec(), leaf(ranges_cbor(&[(&shard_b, &hi())]))),
                ),
            ),
        ),
        delegation_tree(&SUBNET, &ranges_cbor(&[]), &[0], NOW),
    );
    assert_eq!(in_ranges(&tree, &SUBNET, &CANISTER), Ok(true));
    assert_eq!(
        in_ranges(&tree, &SUBNET, &[0, 0, 0, 0, 0, 0x01, 0, 0, 1, 1]),
        Ok(true)
    );
    assert_eq!(
        in_ranges(&tree, &SUBNET, &[0, 0, 0, 0, 0, 0x20, 0, 0, 1, 1]),
        Ok(false)
    );
    assert_eq!(
        in_ranges(&tree, &SUBNET, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Err(Error::Certificate("canister outside delegation"))
    );
}

#[test]
fn ranges_compare_principals_by_length_first() {
    let tree = delegation_tree(&SUBNET, &ranges_cbor(&[(&[5; 10], &[5; 10])]), &[0], NOW);
    assert_eq!(in_ranges(&tree, &SUBNET, &[5; 10]), Ok(true));
    assert_eq!(in_ranges(&tree, &SUBNET, &[5; 9]), Ok(false));
    assert_eq!(in_ranges(&tree, &SUBNET, &[5; 11]), Ok(false));
    assert_eq!(
        in_ranges(&tree, &SUBNET, &[5; 30]),
        Err(Error::Certificate("principal"))
    );
    let bad = delegation_tree(&SUBNET, &[0xff], &[0], NOW);
    assert_eq!(
        in_ranges(&bad, &SUBNET, &CANISTER),
        Err(Error::Certificate("canister ranges"))
    );
    assert!(in_ranges(&tree, &[1; 29], &CANISTER).is_err());
}

#[test]
fn verification_is_deterministic() {
    let root = TestSubnet::new(4);
    let cert = root.certificate(state_tree(&CANISTER, &[1; 32], NOW), None);
    let a = verify(&cert, &root.public_der(), &CANISTER, NOW, 0);
    assert_eq!(a, verify(&cert, &root.public_der(), &CANISTER, NOW, 0));
    assert_eq!(
        cert,
        root.certificate(state_tree(&CANISTER, &[1; 32], NOW), None)
    );
}

#[test]
fn ic_root_key_is_the_published_mainnet_key() {
    assert_eq!(
        hex::encode(IC_ROOT_KEY_DER),
        "308182301d060d2b0601040182dc7c0503010201060c2b0601040182dc7c05030201036100814c0e6ec71fab583b08bd81373c255c3c371b2e84863c98a4f1e08b74235d14fb5d9c0cd546d9685f913a0c0b2cc5341583bf4b4392e467db96d65b9bb4cb717112f8472e0d5a4d14505ffd7484b01291091c5f87b98883463f98091a0baaae"
    );
    assert_eq!(raw_key(&IC_ROOT_KEY_DER).unwrap().len(), RAW_KEY_LEN);
    let root = TestSubnet::new(1);
    let cert = root.certificate(state_tree(&CANISTER, &[5; 32], NOW), None);
    assert_eq!(
        verify(&cert, &IC_ROOT_KEY_DER, &CANISTER, NOW, 0),
        Err(Error::Certificate("signature"))
    );
}
