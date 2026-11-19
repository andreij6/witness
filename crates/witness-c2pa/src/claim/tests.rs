use super::*;
use crate::testkit::Rng;
use alloc::vec;

const M: &str = "urn:c2pa:5a5a5a5a-5a5a-4a5a-9a5a-5a5a5a5a5a5a";
const GEN: Generator<'static> = Generator {
    name: "Witness",
    version: "0.1.0",
};

fn enc(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
    let mut v = Vec::new();
    f(&mut Encoder::new(&mut v));
    v
}

fn sample() -> Vec<u8> {
    let a = assertion_uri(M, "c2pa.actions.v2");
    let b = assertion_uri(M, "app.witness.claim");
    let mut out = Vec::new();
    write_claim(
        &mut out,
        "xmp:iid:1",
        &GEN,
        &signature_uri(M),
        &[
            HashedUri {
                url: &a,
                alg: None,
                hash: &[1; 32],
            },
            HashedUri {
                url: &b,
                alg: Some("sha256"),
                hash: &[2; 32],
            },
        ],
    );
    out
}

#[test]
fn uuid_string_sets_version_and_variant() {
    let s = uuid_v4_string(&[0x5a; 16]);
    assert_eq!(s, "5a5a5a5a-5a5a-4a5a-9a5a-5a5a5a5a5a5a");
    let s = uuid_v4_string(&[0xff; 16]);
    assert_eq!(s, "ffffffff-ffff-4fff-bfff-ffffffffffff");
    let s = uuid_v4_string(&[0; 16]);
    assert_eq!(s, "00000000-0000-4000-8000-000000000000");
    let mut rng = Rng::new(1);
    for _ in 0..200 {
        let id: [u8; 16] = rng.bytes(16).try_into().unwrap();
        let s = uuid_v4_string(&id);
        assert_eq!(s.len(), 36);
        assert_eq!(s.as_bytes()[14], b'4');
        assert!(matches!(s.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        assert_eq!(
            s.char_indices()
                .filter(|(_, c)| *c == '-')
                .map(|(i, _)| i)
                .collect::<Vec<_>>(),
            vec![8, 13, 18, 23]
        );
    }
}

#[test]
fn labels_use_expected_prefixes() {
    assert_eq!(manifest_label(&[0x5a; 16]), M);
    assert_eq!(
        instance_id(&[0x5a; 16]),
        "xmp:iid:5a5a5a5a-5a5a-4a5a-9a5a-5a5a5a5a5a5a"
    );
}

#[test]
fn uris_round_trip() {
    let u = assertion_uri(M, "c2pa.hash.data");
    assert_eq!(
        u,
        alloc::format!("self#jumbf=/c2pa/{M}/c2pa.assertions/c2pa.hash.data")
    );
    assert_eq!(assertion_label(&u, M), Some("c2pa.hash.data"));
    assert_eq!(
        assertion_label("self#jumbf=c2pa.assertions/c2pa.actions.v2", M),
        Some("c2pa.actions.v2")
    );
    let s = signature_uri(M);
    assert!(is_signature_uri(&s, M));
    assert!(is_signature_uri("self#jumbf=c2pa.signature", M));
    assert_eq!(signature_manifest(&s), Some(M));
    assert_eq!(signature_manifest("self#jumbf=c2pa.signature"), Some(""));
}

#[test]
fn foreign_or_malformed_uris_do_not_resolve() {
    for bad in [
        "self#jumbf=/c2pa/urn:c2pa:other/c2pa.assertions/c2pa.hash.data".into(),
        alloc::format!("self#jumbf=/c2pa/{M}/c2pa.assertions/"),
        alloc::format!("self#jumbf=/c2pa/{M}/c2pa.assertions/a/b"),
        alloc::format!("self#jumbf=/c2pa/{M}/c2pa.assertionsX/a"),
        alloc::format!("self#jumbf=/other/{M}/c2pa.assertions/a"),
        alloc::format!("/c2pa/{M}/c2pa.assertions/a"),
        alloc::format!("self#jumbf=/c2pa/{M}c2pa.assertions/a"),
        "self#jumbf=c2pa.assertions".into(),
        String::new(),
    ] {
        assert_eq!(assertion_label(&bad, M), None, "{bad}");
    }
    assert!(!is_signature_uri(
        "self#jumbf=/c2pa/urn:c2pa:other/c2pa.signature",
        M
    ));
    assert!(!is_signature_uri(&assertion_uri(M, "c2pa.signature"), M));
    for bad in [
        "self#jumbf=/c2pa//c2pa.signature",
        "self#jumbf=/c2pa/a/b/c2pa.signature",
        "self#jumbf=c2pa.signatureX",
        "self#jumbf=/c2pa/a/c2pa.claim",
        "c2pa.signature",
    ] {
        assert_eq!(signature_manifest(bad), None, "{bad}");
    }
}

#[test]
fn claim_round_trips() {
    let bytes = sample();
    let c = ClaimV2::parse(&bytes).unwrap();
    assert_eq!(c.instance_id, "xmp:iid:1");
    assert_eq!(c.generator_name, "Witness");
    assert_eq!(c.signature, signature_uri(M));
    assert_eq!(c.alg, Some(ALG_SHA256));
    assert_eq!(c.created.len(), 2);
    assert_eq!(c.created[0].alg, None);
    assert_eq!(c.created[1].alg, Some("sha256"));
    assert_eq!(c.created[1].hash, &[2; 32]);
    assert!(c.gathered.is_empty());
    assert_eq!(c.all_assertions().count(), 2);
}

#[test]
fn claim_write_is_deterministic() {
    assert_eq!(sample(), sample());
}

type Field<'a> = (&'a str, &'a dyn Fn(&mut Encoder<'_>));

fn claim_with(entries: &[Field<'_>]) -> Vec<u8> {
    enc(|e| {
        e.map(entries.len());
        for (k, v) in entries {
            e.text(k);
            v(e);
        }
    })
}

fn uri_list(e: &mut Encoder<'_>) {
    e.array(1);
    e.map(2);
    e.text("url");
    e.text("self#jumbf=c2pa.assertions/x");
    e.text("hash");
    e.bytes(&[0; 32]);
}

fn text(s: &'static str) -> impl Fn(&mut Encoder<'_>) {
    move |e| e.text(s)
}

fn generator(e: &mut Encoder<'_>) {
    e.map(3);
    e.text("name");
    e.text("Other");
    e.text("icon");
    e.map(0);
    e.text("version");
    e.text("2");
}

#[test]
fn claim_accepts_optional_fields() {
    let iid = text("iid");
    let sig = text("self#jumbf=c2pa.signature");
    let title = text("title");
    let soft = text("soft");
    let meta = |e: &mut Encoder<'_>| e.map(0);
    let bytes = claim_with(&[
        ("instanceID", &iid),
        ("claim_generator_info", &generator),
        ("signature", &sig),
        ("created_assertions", &uri_list),
        ("gathered_assertions", &uri_list),
        ("dc:title", &title),
        ("alg_soft", &soft),
        ("metadata", &meta),
    ]);
    let c = ClaimV2::parse(&bytes).unwrap();
    assert_eq!(c.generator_name, "Other");
    assert_eq!(c.gathered.len(), 1);
    assert_eq!(c.all_assertions().count(), 2);
    assert_eq!(c.alg, None);
}

#[test]
fn claim_rejects_missing_or_unknown_fields() {
    let iid = text("iid");
    let sig = text("self#jumbf=c2pa.signature");
    let empty = |e: &mut Encoder<'_>| e.array(0);
    let red = |e: &mut Encoder<'_>| {
        e.array(1);
        e.text("self#jumbf=x");
    };
    let all: [Field<'_>; 4] = [
        ("instanceID", &iid),
        ("claim_generator_info", &generator),
        ("signature", &sig),
        ("created_assertions", &uri_list),
    ];
    for skip in 0..4 {
        let entries: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, e)| *e)
            .collect();
        assert_eq!(
            ClaimV2::parse(&claim_with(&entries)),
            Err(Error::MalformedClaim)
        );
    }
    let mut extra = all.to_vec();
    extra.push(("assertions", &uri_list));
    assert_eq!(
        ClaimV2::parse(&claim_with(&extra)),
        Err(Error::MalformedClaim)
    );
    let mut dup = all.to_vec();
    dup.push(("signature", &sig));
    assert_eq!(
        ClaimV2::parse(&claim_with(&dup)),
        Err(Error::MalformedClaim)
    );
    let mut redacted = all.to_vec();
    redacted.push(("redacted_assertions", &red));
    assert_eq!(
        ClaimV2::parse(&claim_with(&redacted)),
        Err(Error::UnsupportedClaim)
    );
    let mut no_created = all.to_vec();
    no_created[3] = ("created_assertions", &empty);
    assert_eq!(
        ClaimV2::parse(&claim_with(&no_created)),
        Err(Error::MalformedClaim)
    );
    let no_name = |e: &mut Encoder<'_>| {
        e.map(1);
        e.text("version");
        e.text("1");
    };
    let mut nameless = all.to_vec();
    nameless[1] = ("claim_generator_info", &no_name);
    assert_eq!(
        ClaimV2::parse(&claim_with(&nameless)),
        Err(Error::MalformedClaim)
    );
}

#[test]
fn hashed_uri_rejects_bad_maps() {
    let iid = text("iid");
    let sig = text("self#jumbf=c2pa.signature");
    let bad_lists: [&dyn Fn(&mut Encoder<'_>); 4] = [
        &|e| {
            e.array(1);
            e.map(1);
            e.text("url");
            e.text("u");
        },
        &|e| {
            e.array(1);
            e.map(3);
            e.text("url");
            e.text("u");
            e.text("hash");
            e.bytes(&[]);
            e.text("bogus");
            e.u64(1);
        },
        &|e| {
            e.array(1);
            e.map(2);
            e.text("url");
            e.u64(1);
            e.text("hash");
            e.bytes(&[]);
        },
        &|e| {
            e.array(1);
            e.map(3);
            e.text("url");
            e.text("u");
            e.text("url");
            e.text("u");
            e.text("hash");
            e.bytes(&[]);
        },
    ];
    for (i, list) in bad_lists.iter().enumerate() {
        let bytes = claim_with(&[
            ("instanceID", &iid),
            ("claim_generator_info", &generator),
            ("signature", &sig),
            ("created_assertions", *list),
        ]);
        assert!(ClaimV2::parse(&bytes).is_err(), "case {i}");
    }
}

#[test]
fn claim_rejects_too_many_uris_and_trailing_bytes() {
    let iid = text("iid");
    let sig = text("self#jumbf=c2pa.signature");
    let many = |e: &mut Encoder<'_>| {
        e.array(1025);
        for _ in 0..1025 {
            e.map(2);
            e.text("url");
            e.text("u");
            e.text("hash");
            e.bytes(&[]);
        }
    };
    let bytes = claim_with(&[
        ("instanceID", &iid),
        ("claim_generator_info", &generator),
        ("signature", &sig),
        ("created_assertions", &many),
    ]);
    assert_eq!(ClaimV2::parse(&bytes), Err(Error::MalformedClaim));
    let mut trailing = sample();
    trailing.push(0);
    assert_eq!(ClaimV2::parse(&trailing), Err(Error::MalformedCbor));
    assert!(ClaimV2::parse(&[0x80]).is_err());
    assert!(ClaimV2::parse(&[]).is_err());
}

#[test]
fn random_mutations_never_panic() {
    let base = sample();
    let mut rng = Rng::new(2);
    for _ in 0..3000 {
        let mut d = base.clone();
        for _ in 0..1 + rng.below(3) {
            let i = rng.below(d.len());
            d[i] = rng.next() as u8;
        }
        let _ = ClaimV2::parse(&d);
    }
}
