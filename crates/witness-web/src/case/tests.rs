use super::*;
use crate::testkit::seal_case;
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};

const KEY: [u8; 32] = [0x42; 32];

fn case_id() -> CaseId {
    CaseId::new(3, [0xab; 15])
}

fn fragment() -> String {
    format!("#k={}", URL_SAFE_NO_PAD.encode(KEY))
}

fn bundle(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 % 251) as u8).collect()
}

fn opener(b: &[u8], count: usize) -> CaseOpener {
    CaseOpener::new(
        &fragment(),
        &case_id().to_string(),
        count as u32,
        &sha256(b),
    )
    .unwrap()
}

fn open(b: &[u8], chunks: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut o = opener(b, chunks.len());
    for (ix, c) in chunks.iter().enumerate() {
        o.add(ix as u32, c)?;
    }
    o.finish()
}

#[test]
fn fragment_key_parses_padded_unpadded_and_with_params() {
    assert_eq!(key_from_fragment(&fragment()), Ok(KEY));
    assert_eq!(
        key_from_fragment(&format!("k={}", URL_SAFE.encode(KEY))),
        Ok(KEY)
    );
    assert_eq!(
        key_from_fragment(&format!("#x=1&k={}&y=2", URL_SAFE_NO_PAD.encode(KEY))),
        Ok(KEY)
    );
    assert_eq!(key_from_fragment("#x=1"), Err(Error::Case("key missing")));
    assert_eq!(
        key_from_fragment("#k=***"),
        Err(Error::Case("key encoding"))
    );
    assert_eq!(
        key_from_fragment(&format!("#k={}", URL_SAFE_NO_PAD.encode([1; 31]))),
        Err(Error::Case("key length"))
    );
    assert_eq!(key_from_fragment(""), Err(Error::Case("key missing")));
}

#[test]
fn chunk_layout_matches_the_client_gcm_vector() {
    let key: [u8; 32] =
        hex::decode("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308")
            .unwrap()
            .try_into()
            .unwrap();
    let plain = hex::decode(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    )
    .unwrap();
    let aad = hex::decode("feedfacedeadbeeffeedfacedeadbeefabaddad2").unwrap();
    let chunk = hex::decode(concat!(
        "cafebabefacedbaddecaf888",
        "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa",
        "8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662",
        "76fc6ece0f4e1768cddf8853bb2d551b"
    ))
    .unwrap();
    let cipher = Aes256Gcm::new(&key.into());
    let mut out = vec![0; chunk.len() - OVERHEAD];
    decrypt_into(&cipher, &aad, &chunk, &mut out).unwrap();
    assert_eq!(out, plain);
    for i in 0..chunk.len() {
        let mut bad = chunk.clone();
        bad[i] ^= 1;
        assert!(
            decrypt_into(&cipher, &aad, &bad, &mut out).is_err(),
            "byte {i}"
        );
    }
    assert!(decrypt_into(&cipher, &aad[1..], &chunk, &mut out).is_err());
}

#[test]
fn aad_layout_matches_the_client() {
    let a = aad(&case_id(), 2, 5, &[9; 32]);
    assert_eq!(a.len(), 71);
    assert_eq!(&a[..15], b"witness/case/v1");
    assert_eq!(&a[15..31], case_id().as_bytes());
    assert_eq!(&a[31..35], &[0, 0, 0, 2]);
    assert_eq!(&a[35..39], &[0, 0, 0, 5]);
    assert_eq!(&a[39..], &[9; 32]);
}

#[test]
fn single_and_multi_chunk_bundles_open() {
    for len in [
        0,
        1,
        1000,
        PLAIN_CHUNK,
        PLAIN_CHUNK + 1,
        2 * PLAIN_CHUNK + 17,
    ] {
        let b = bundle(len);
        let chunks = seal_case(&KEY, &case_id(), &b, 1);
        assert_eq!(chunks.len(), len.div_ceil(PLAIN_CHUNK).max(1));
        assert_eq!(open(&b, &chunks), Ok(b), "{len}");
    }
}

#[test]
fn chunks_may_arrive_in_any_order() {
    let b = bundle(2 * PLAIN_CHUNK + 5);
    let chunks = seal_case(&KEY, &case_id(), &b, 2);
    let mut o = opener(&b, chunks.len());
    for ix in [2usize, 0, 1] {
        o.add(ix as u32, &chunks[ix]).unwrap();
    }
    assert!(o.missing().is_empty());
    assert_eq!(o.finish(), Ok(b));
}

#[test]
fn missing_chunks_are_reported_and_block_finish() {
    let b = bundle(PLAIN_CHUNK + 5);
    let chunks = seal_case(&KEY, &case_id(), &b, 3);
    let mut o = opener(&b, chunks.len());
    assert_eq!(o.missing(), [0, 1]);
    o.add(1, &chunks[1]).unwrap();
    assert_eq!(o.missing(), [0]);
    assert_eq!(o.finish(), Err(Error::Case("missing chunk")));
}

#[test]
fn tampered_or_misplaced_chunks_fail_authentication() {
    let b = bundle(PLAIN_CHUNK + 100);
    let chunks = seal_case(&KEY, &case_id(), &b, 4);
    let mut flipped = chunks.clone();
    flipped[1][20] ^= 1;
    assert_eq!(open(&b, &flipped), Err(Error::Case("authentication")));
    let mut nonce = chunks.clone();
    nonce[0][0] ^= 1;
    assert_eq!(open(&b, &nonce), Err(Error::Case("authentication")));
    let mut tag = chunks.clone();
    let last = tag[1].len() - 1;
    tag[1][last] ^= 1;
    assert_eq!(open(&b, &tag), Err(Error::Case("authentication")));
    let mut o = opener(&b, 2);
    assert_eq!(o.add(0, &chunks[1]), Err(Error::Case("chunk size")));
    assert_eq!(o.add(1, &chunks[0]), Err(Error::Case("authentication")));
    let other_case = CaseId::new(3, [0xac; 15]);
    let foreign = seal_case(&KEY, &other_case, &b, 4);
    assert_eq!(open(&b, &foreign), Err(Error::Case("authentication")));
    let mut wrong_key = opener(&b, 2);
    let alien = seal_case(&[7; 32], &case_id(), &b, 4);
    assert_eq!(
        wrong_key.add(0, &alien[0]),
        Err(Error::Case("authentication"))
    );
}

#[test]
fn a_bad_retry_invalidates_an_accepted_chunk() {
    let b = bundle(100);
    let chunks = seal_case(&KEY, &case_id(), &b, 5);
    let mut o = opener(&b, 1);
    o.add(0, &chunks[0]).unwrap();
    let mut bad = chunks[0].clone();
    bad[15] ^= 1;
    assert!(o.add(0, &bad).is_err());
    assert_eq!(o.missing(), [0]);
    o.add(0, &chunks[0]).unwrap();
    assert_eq!(o.finish(), Ok(b));
}

#[test]
fn wrong_bundle_hash_or_count_is_rejected() {
    let b = bundle(500);
    let chunks = seal_case(&KEY, &case_id(), &b, 6);
    let mut o = CaseOpener::new(&fragment(), &case_id().to_string(), 1, &[0; 32]).unwrap();
    assert_eq!(o.add(0, &chunks[0]), Err(Error::Case("authentication")));
    let mut o = opener(&b, 2);
    assert_eq!(o.add(1, &chunks[0]), Err(Error::Case("authentication")));
}

fn seal_one(count: u32, ix: u32, plain: &[u8], hash: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(&KEY.into());
    let nonce = [ix as u8; NONCE_BYTES];
    let mut body = plain.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(
            Nonce::from_slice(&nonce),
            &aad(&case_id(), ix, count, hash),
            &mut body,
        )
        .unwrap();
    [nonce.as_slice(), &body, &tag].concat()
}

#[test]
fn plus_sized_cases_track_chunks_past_the_first_word() {
    let h = [7u8; 32];
    let count = MAX_PLUS_CASE_CHUNKS;
    let mut o = CaseOpener::new(&fragment(), &case_id().to_string(), count, &h).unwrap();
    assert_eq!(o.missing().len(), count as usize);
    let last = count - 1;
    assert_eq!(o.add(last, &seal_one(count, last, b"tail", &h)), Ok(()));
    let missing = o.missing();
    assert_eq!(missing.len(), count as usize - 1);
    assert!(!missing.contains(&last));
    assert_eq!(missing.last(), Some(&(last - 1)));
    let mut wrong = seal_one(count, last, b"tail", &h);
    wrong[NONCE_BYTES] ^= 1;
    assert_eq!(o.add(last, &wrong), Err(Error::Case("authentication")));
    assert!(o.missing().contains(&last));
    assert_eq!(
        o.add(64, &seal_one(count, 64, b"x", &h)),
        Err(Error::Case("chunk size"))
    );
    assert_eq!(
        o.add(count, &seal_one(count, last, b"x", &h)),
        Err(Error::Case("chunk index"))
    );
}

#[test]
fn caps_cover_free_and_plus_cases() {
    assert_eq!(
        (MAX_CASE_CHUNKS, MAX_PLUS_CASE_CHUNKS, MAX_CHUNKS),
        (16, 256, 256)
    );
    assert_eq!(MAX_WIRE_CHUNK, 1 << 20);
}

#[test]
fn constructor_validates_inputs() {
    let id = case_id().to_string();
    let h = [0u8; 32];
    assert!(CaseOpener::new(&fragment(), &id, 1, &h).is_ok());
    assert!(CaseOpener::new(&fragment(), &id, MAX_CHUNKS, &h).is_ok());
    assert_eq!(
        CaseOpener::new(&fragment(), &id, 0, &h).err(),
        Some(Error::Case("chunk count"))
    );
    assert_eq!(
        CaseOpener::new(&fragment(), &id, MAX_CHUNKS + 1, &h).err(),
        Some(Error::Case("chunk count"))
    );
    assert_eq!(
        CaseOpener::new(&fragment(), "short", 1, &h).err(),
        Some(Error::Case("case id"))
    );
    assert_eq!(
        CaseOpener::new(&fragment(), &id, 1, &h[..31]).err(),
        Some(Error::Case("bundle hash"))
    );
    assert_eq!(
        CaseOpener::new("#", &id, 1, &h).err(),
        Some(Error::Case("key missing"))
    );
}

#[test]
fn chunk_bounds_are_enforced() {
    let mut o = CaseOpener::new(&fragment(), &case_id().to_string(), 2, &[0; 32]).unwrap();
    assert_eq!(o.add(2, &[0; 64]), Err(Error::Case("chunk index")));
    assert_eq!(o.add(1, &[0; OVERHEAD - 1]), Err(Error::Case("chunk size")));
    assert_eq!(
        o.add(1, &vec![0; MAX_WIRE_CHUNK + 1]),
        Err(Error::Case("chunk size"))
    );
    assert_eq!(o.add(0, &[0; 64]), Err(Error::Case("chunk size")));
    assert_eq!(o.add(1, &[0; OVERHEAD]), Err(Error::Case("authentication")));
}

#[test]
fn opening_is_deterministic() {
    let b = bundle(3000);
    let chunks = seal_case(&KEY, &case_id(), &b, 7);
    assert_eq!(chunks, seal_case(&KEY, &case_id(), &b, 7));
    assert_eq!(open(&b, &chunks), open(&b, &chunks));
}

const KOTLIN_BUNDLE: &str = concat!(
    "a40001016b526561722062756d706572026a74776f2070686f746f730382a2004a010101010101010101010145ffd8010203a3004add1b3c312cf7d81613030159012c00",
    "070e151c232a31383f464d545b626970777e858c939aa1a8afb6bdc4cbd2d9e0e7eef501080f161d242b323940474e555c636a71787f868d949ba2a9b0b7bec5ccd3dae1e8eff6",
    "020910171e252c333a41484f565d646b727980878e959ca3aab1b8bfc6cdd4dbe2e9f0f7030a11181f262d343b424950575e656c737a81888f969da4abb2b9c0c7ced5dce3eaf1f8",
    "040b121920272e353c434a51585f666d747b828990979ea5acb3bac1c8cfd6dde4ebf2f9050c131a21282f363d444b525960676e757c838a91989fa6adb4bbc2c9d0d7dee5ecf3fa",
    "060d141b222930373e454c535a61686f767d848b9299a0a7aeb5bcc3cad1d8dfe6edf400070e151c232a31383f464d545b626970777e858c939aa1a8afb6bdc4cbd2d9e0e7eef501",
    "080f161d242b323940474e5502a400010145636c61696d0244633270610344636f7365",
);

const KOTLIN_CHUNK: &str = concat!(
    "070707070707070707070707dce25caddea3179a51748e45b02b430ef7307b4a92f5382d0f00502019afba7465e91bd175b2744c46f744899f60b854846c1c296604e67db1",
    "29a2f6040b26c9399481debfd79b3b9aff4c6ca54a1b417c5f59232306dc763cedbe6e3fd646923c634e3096f61f8f56bf81cc31d9930ff9b046b218fcb1160473f71d7b2a0ae887",
    "4c76d319732c755719c1d07fedf0a19f43a5a9bb9ff5301caa3e22dd6f3d7f37e55b27a2efcc8406ef62e1065f08fad2cdc38ffe18cf46effe794e879fbcb5f8a01b2a14c330d624",
    "e78aa7c4344a5ba027829053eaf8c7f43fbf21a79ae329ca8a7e2da81344aec9a41274b98063f602981a0779b80f228283917619da7f26ce2274bfd5c78723d727a70620bc260d4f",
    "6864af266057c0390b9482f72445e1a499c7c087618230e0c14d14b15ccf956c4689ac99155246c4f27fafb0be127ab294ec593244ca1497b343237a298caed6e5ada562e6c9596b",
    "a4643283c1aa24433fe99e1590ce9816c7903a14ce87f8f4131159c3b74ee028221d685a75148871a355e9e52a3191a9e012ac122f016f57ab683be3ea",
);

#[test]
fn kotlin_share_case_vector_opens_and_parses() {
    let id = CaseId::new(0, [0xab; 15]);
    assert_eq!(id.to_string(), "02NTQAXBNENTQAXBNENTQAXBNC");
    let bundle = hex::decode(KOTLIN_BUNDLE).unwrap();
    let hash = sha256(&bundle);
    assert_eq!(
        hex::encode(hash),
        "e94360c3290ec996e280f1ed75ea40525ffa37232abebb9e0b93da9dd5ab3f00"
    );
    let fragment = "#k=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI";
    assert_eq!(fragment, self::fragment());
    let mut o = CaseOpener::new(fragment, &id.to_string(), 1, &hash).unwrap();
    o.add(0, &hex::decode(KOTLIN_CHUNK).unwrap()).unwrap();
    let opened = o.finish().unwrap();
    assert_eq!(opened, bundle);
    let parsed = crate::bundle::CaseBundle::parse(&opened).unwrap();
    assert_eq!(parsed.title, "Rear bumper");
    assert_eq!(parsed.summary, Some("two photos"));
    assert_eq!(parsed.items.len(), 2);
    assert_eq!(parsed.items[0].jpeg, &[0xff, 0xd8, 1, 2, 3]);
    assert!(parsed.items[0].submission.is_none());
    assert_eq!(parsed.items[1].record_id.to_string(), "VMDKRC9CYZC1C4R3");
    let s = parsed.items[1].submission.as_ref().unwrap();
    assert_eq!(s.claim_cbor, b"claim");
    assert_eq!(parsed.items[1].jpeg.len(), 300);
    assert_eq!(parsed.to_cbor(), bundle);
    assert_eq!(
        seal_case(&KEY, &id, &bundle, 7)[0],
        hex::decode(KOTLIN_CHUNK).unwrap()
    );
}
