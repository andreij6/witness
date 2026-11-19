

# ---- crate report ----
I built the `witness-image` crate, and all three required checks pass. `cargo test -p witness-image` runs 186 tests: 143 unit and 43 integration. `cargo clippy -p witness-image --all-targets -- -D warnings` is clean. `cargo build -p witness-image --target wasm32-unknown-unknown` succeeds in debug and release. I only changed `crates/witness-image/`. There are no code comments and no `unsafe`. The wasm32 dependency tree has no getrandom, wasm-bindgen, tokio or js-sys.

**Dependencies (exact pins where a decoder or encoder is involved):** `zune-jpeg =0.5.15`, `zune-png =0.5.2`, `zune-core 0.5`, `image-webp =0.2.4`, `jpeg-encoder =0.7.1`, `sha2 0.10`. Default features are off on all of them, so the NEON/x86/AVX2 code is not compiled in.

## Public API (`src/lib.rs` re-exports)

**Buffers (`buffer.rs`)**
- `Image<const C>`, with aliases `Rgb8 = Image<3>` and `Luma8 = Image<1>`: a contiguous row-major `Vec<u8>`.
  - Constructors: `new`, `filled`, `from_fn`.
  - Accessors: `width`, `height`, `dimensions`, `data`, `data_mut`, `into_data`, `row`, `row_mut`, `pixel`.
  - `view` and `crop` return zero-copy views.
- `View<'a, C>`, with aliases `RgbView` and `LumaView`: a borrowed view with a stride.
  - `new` (validated), `row`, `rows`, `pixel`, `crop` (zero-copy, can be cropped again), `is_contiguous`, `to_image`.
- Zero width or height is rejected everywhere.

**Formats and decoding**
- `Format` and `sniff(bytes)`: detects JPEG, PNG or WebP from the magic bytes.
- `decode` and `decode_with_limit`: decode to RGB8. Anything else returns `ImageError::UnsupportedFormat`. The default limit is `DEFAULT_MAX_PIXELS` = 2²⁶.
- `decode_oriented` and `decode_oriented_with_limit`: decode, then apply the JPEG EXIF orientation.
- `orientation_of(bytes)`: the orientation the oriented decode would apply.

**EXIF**
- `jpeg_orientation`, `exif_payload`, `tiff_orientation`: my own segment walker and IFD0 parser.
  - Bounds-checked with checked arithmetic, so it never panics.
  - Accepts SHORT or LONG values; the first Orientation tag wins.

**Orientation**
- `Orientation`: the 8 EXIF orientations, with `from_exif`, `exif_value`, `inverse` and `swaps_dimensions`.
- `orient(view, o)` plus `rotate90`, `rotate180`, `rotate270`, `flip_horizontal`, `flip_vertical`, `transpose`.

**Luma**
- `luma_bt601`, `to_luma` and `luma_to_rgb`, using the fixed-point formula `(19595R + 38470G + 7471B + 2¹⁵) >> 16`.

**Resampling**
- `jarosz_window`: ceil(src / 2dst).
- `jarosz_sample_index`: PDQ's sample centres, `floor((i + 0.5)·src/dst)`.
- `jarosz_downsample_q8`: returns `Vec<u16>` in Q8.8 fixed point; `Q8_ONE` is 256.
- `jarosz_downsample`: the same result rounded to `Luma8`.
- `resize_area`: exact area average using integer weights, rounded once.
- `resize_bilinear`: half-pixel centres with 11-bit fixed-point weights (`BILINEAR_FRACTION_BITS`).

**Encoding, stripping and hashing**
- `encode_jpeg(&Rgb8, quality)`: quality is clamped to 1–100.
- `strip_and_normalize_jpeg`:
  - Keeps the JFIF APP0 and the Adobe APP14 segments.
  - Drops every other APPn segment (including EXIF/XMP, ICC/MPF and APP11), comments and fill bytes.
  - Cuts the file at the first end-of-image marker, which removes Ultra HDR gain maps.
  - Rejects a file with no frame or a truncated file.
- `pixel_hash`, `pixel_hash_of(view)` and the `PixelHash` type: SHA-256 streamed over the rows.

## Complexity
- **Rotations and flips:** O(n). Row-preserving cases copy rows directly; the transposing cases use 64×64 tiles to stay cache-friendly.
- **Jarosz downsample:** O(W·H) time and O(H·dst_w + W) memory. Each source row is filtered horizontally twice, then only the dst_w sampled columns are kept for the two vertical passes. The result is identical to filtering the full image and then decimating, and a test checks this.
- **Area resize:** O(W·H + dst) time. Each source row's horizontal pass is computed once and cached, so memory is two output rows.
- **Bilinear resize:** O(dst) time, keeping a cache of two interpolated rows.
- **EXIF, segment walking and stripping:** single pass, O(bytes).
- **Format conversion:** RGBA, grey and grey+alpha are converted to RGB in place in the decoder's own buffer.

## Tests
They cover the following:
- PDQ's float box filter and a naive reference.
- Bilinear against a float reference within ±1.
- Exact area weights against a brute-force reference.
- All 8 orientations on asymmetric shapes and strided crops, plus inverse and quarter-turn checks.
- JPEG, PNG and WebP round trips:
  - PNG: 1/2/16-bit, palette with tRNS, grey+alpha, built with a test-only stored-deflate writer.
  - WebP: lossless, plus lossy fixtures made with `cwebp`.
  - JPEG: progressive, restart intervals, greyscale.
- Encoding JPEGs with EXIF orientations 1–8, then checking the decoded quadrant colours.
- A table of malformed EXIF inputs.
- Several thousand mutated or truncated files per format: nothing panics.
- Size limits, and a 12 MP Jarosz smoke run.
- Golden SHA-256 values pinned in `tests/golden.rs`.

## Deviations and things to know
1. **`pixel_hash` also covers the dimensions.** It is SHA-256(width as big-endian u32 ‖ height as big-endian u32 ‖ RGB bytes), not the RGB bytes alone. Without this, a 2×3 image and a 3×2 image with the same bytes would hash the same, which matters for an "Original" verdict. If you want the plain spec version, it is a one-line change in `src/hash.rs`.
2. **`Image` fields are private, with accessors**, rather than a public `Rgb8 { width, height, data }`. This keeps `data.len() == w·h·C` always true. Use `Rgb8::new(w, h, data)`, which checks the length.
3. **Feature unification can break determinism.** Turning off default features is not enough on its own: zune's default options enable its unsafe/SIMD code, and `set_use_unsafe(false)` does not clear the NEON flag. If any other crate in the workspace enables zune-jpeg's default `neon`/`x86` or jpeg-encoder's `simd` features, the SIMD code is compiled back in. I suggest adding this to the CI dependency check.
4. **Alpha is dropped, not blended over a background.** A PNG with fewer than 8 bits per sample is scaled to the full 0–255 range by zune.
5. **EXIF orientation is applied for JPEG only, as specified.** PNG eXIf and WebP EXIF chunks are ignored.
6. **Fixed-point results differ slightly from a float pipeline.** Jarosz runs both horizontal passes before both vertical passes and rounds to Q8 after each pass. It stays within ±0.5 of PDQ's float filter per pass, but it is not bit-identical to the float PDQ reference.
7. **The resizers also upscale.** `jarosz_*` and `resize_area` accept targets larger than the source, which they handle correctly.
8. **`encode_jpeg` uses jpeg-encoder's defaults:** 4:2:0 subsampling below quality 90, and 4:4:4 at 90 and above.
9. **The golden values were only run on aarch64-apple-darwin.** They have not been run on wasm32; the cross-target check belongs in witness-core's golden CI.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-image/`:
- `Cargo.toml`
- `src/`: `lib.rs`, `buffer.rs`, `decode.rs`, `encode.rs`, `error.rs`, `exif.rs`, `format.rs`, `hash.rs`, `jpeg_markers.rs`, `luma.rs`, `orient.rs`, `strip.rs`, and `resample/` (`mod.rs`, `jarosz.rs`, `area.rs`, `bilinear.rs`)
- `tests/`: `common/mod.rs`, `decode.rs`, `orientation.rs`, `strip.rs`, `robustness.rs`, `pipeline.rs`, `golden.rs`, and `fixtures/` (`lossy.webp`, `lossy_alpha.webp`)

# ---- crate report ----
witness-format is built. All three checks pass:
- **Tests:** `cargo test -p witness-format` passes: 135 unit tests plus 5 integration tests in `tests/golden.rs`.
- **Clippy:** `cargo clippy -p witness-format --all-targets -- -D warnings` reports nothing.
- **wasm:** `cargo build -p witness-format --target wasm32-unknown-unknown` succeeds in both the dev and `wasm` profiles.

The crate is `#![no_std]` + alloc and has no code comments. The runtime dependencies are sha2 0.10.9, p256 0.13.2 (`ecdsa`, `pkcs8`), ecdsa 0.16.9 (`verifying`, `der`), ed25519-dalek 2.2.0 and subtle 2.6.1, all with default features off. The wasm32 dependency tree contains no getrandom, wasm-bindgen, js-sys or tokio; rand_core is present but without an entropy source. I edited only `crates/witness-format/`; cargo also created the workspace `Cargo.lock`.

## Public API

**`cbor` (canonical CBOR, RFC 8949 §4.2.1)**
- `Decoder<'a>`: reads directly from `&[u8]` without copying. Methods: `u64`, `i64`, `bytes`, `byte_array::<N>`, `text`, `bool`, `null`, `array`, `array_of`, `map`, `fields`, `skip`, `raw_item`, `value`, `peek_major`, `finish`.
- It rejects:
  - indefinite lengths and break bytes
  - floats and tags
  - simple values other than false/true/null
  - reserved additional-info values
  - integers or lengths not in their shortest form
  - duplicate or unordered map keys (compared by encoded bytes)
  - invalid UTF-8
  - nesting deeper than 32
  - declared lengths longer than the remaining input
  - trailing bytes
- `Fields`: reads structs with integer keys: `version`, `req`, `opt`, `key`, `finish`. Keys must strictly increase, and missing or unknown keys are errors.
- `Encoder<'b>`: writes shortest-form heads. Helpers: `field`, `opt_field`, `put`, `raw`.
- `Value<'a>`: generic value that borrows from the input. `to_vec` sorts map keys canonically and rejects duplicates.
- Traits `Encode` and `Decode<'a>`, with implementations for the unsigned and signed integer types, bool, `[u8; N]`, `&[u8]`, `&str`, `String`, `Vec<T>` and `(A, B)`.
- Free functions: `to_vec`, `encode_into` (reuses a buffer), `from_slice` (must consume all input), `from_slice_canonical` (also re-encodes and compares bytes, for the registry's check), `validate`.
- `Error` enum.

**`ids`**
- `CaptureId`: `new(random56, schema5)`, `from_random_bytes`, `from_u64`, `to_u64`, `schema()`, `random()`.
- `RecordId`: `from_claim_cbor` = SHA-256(claim)[..10]. Also `Display`, `FromStr`, `parse`, `to_base32`, `to_grouped`, `grouped()` ("XXXX-XXXX-XXXX-XXXX").
- `KeyId::from_spki` = SHA-256(SPKI)[..16].
- `CaseId::new(shard, [u8; 15])` and `shard()`, with a 26-character Crockford text form.
- `base32`: generic Crockford `encode` and `decode`. Decoding ignores case, maps I/L to 1 and O to 0, ignores hyphens, and rejects non-zero padding bits.

**Claim types (`claim`, `edit`)**
- `WitnessClaim`: keys 0–12 as in §3.2, version 2. Methods: `validate`, `to_cbor`, `from_cbor`, `record_id`.
- Helper types `Dims`, `KeyRef`, `CertSerial` (stored without heap allocation) and `Geohash5`.
- `Kind`: `Original` or `Derivative { parent, root, edits }`, with `edits()` and `presentation_only()`.
- `Submission`: version 1, with `record_id`, `claim` and `within_limits` (each part at most 64 KiB).
- `Edit`: eight kinds, plus `EditKind::class()`, `EditClass`, `BlurClass`, `ImageFormat`, `Rect`, `presentation_only(&[Edit])` and `has_local`.

**`evidence`**
- `TimeEvidence`, with `clock_skew_ms`.
- `LocationEvidence`: `Confidence` plus a `LocationReasons` bitset.
- `location_commit(salt, lat_e7, lon_e7, acc_cm)` and `LocationOpening` with `commit`, `verify` (constant-time compare plus range check) and a CBOR export format. Its `Debug` output hides the coordinates.
- `Checks`: packed into one u32 with these fields:
  - `SecurityLevel` (StrongBox, Tee or Dev)
  - `BootState`
  - device locked
  - mock location
  - `GnssChecks` bitset
  - recapture score (u8)
  - reserved bits must be zero

**`fingerprint`**
- `FingerprintBundle::new` sorts PDQ shapes by tag and rejects duplicate tags. It allows at most 32 shapes and a quality of at most 100.
- Accessors: `shape(tag)` (binary search), `grid_dims`, `cell`, `grid_bytes`, `fp_commit`.

**`key`**
- `KeyInfo`: version 1, with `validate` (`key_id` must match the SPKI; chain serials must be normalized hex), `leaf_hash`, `is_dev` and `accepts_original_at_ms`.
- `KeyHistory`: `new`, `from_changes`, `push`, `status_at`, `active_at`, `revoked_after`, `first_distrusted_after`, `current`.
- Enums `KeyStatus`, `Provisioning`, `Freshness`; helpers `normalize_serial_hex` and `is_normalized_serial_hex`.

**`verdict`**
- `Outcome`: the five outcomes.
- `VerdictFlags`: the seven flags.

**`crypto`**
- Hashing: `sha256`, `sha256_parts`, `hash_ds(domain, parts)` with constants in `domain::*`, and `ct_eq` (via subtle).
- ES256: `Es256Key` (`parse` accepts SEC1 or SPKI DER; `verify`, `verify_prehash`, `verify_low_s`), plus `es256_verify` and `es256_verify_low_s`.
- Signature format: signatures may be raw `r‖s` (64 bytes) or DER. `es256_der_to_raw`, `es256_raw_to_der` (returns a fixed-size `DerSignature`, no allocation), `es256_sig_to_raw`, `es256_is_low_s`, `es256_normalize_low_s`.
- `ed25519_verify` uses `verify_strict`.

## Complexity
- Decoding, `validate` and encoding are all O(n) in one pass with no backtracking. The decoder holds O(1) state per nesting level, with depth capped at 32.
- Map key order is checked by comparing each encoded key's bytes with the previous one, without copying.
- `Value` map encoding is O(k log k) per map and uses a single scratch buffer.
- `KeyHistory::status_at`, `active_at` and `revoked_after` use binary search (`partition_point`): O(log h), plus scanning the entries after that point.
- `FingerprintBundle::shape` is O(log s).
- IDs, `CertSerial`, `DerSignature` and the grid are fixed-size arrays with no heap allocation.

## Tests (140)
- **Golden vectors:**
  - RFC 8949 Appendix A
  - RFC 6979 P-256/SHA-256 for "sample" (a high-S signature) and "test"
  - RFC 8032 Ed25519 tests 1 and 2
  - the SHA-256 FIPS vectors
  - hand-written byte encodings of a full original claim, a derivative's kind, Edits, Checks, time and location evidence, and the fingerprint bundle
  - a pinned record ID: `SYXJWEC36ZHC9FGC`
- **Malformed input:** a table of about 90 bad CBOR inputs, each checked to give the same error from both `validate` and the `Value` decoder.
- **Seeded property tests** (a small built-in PRNG):
  - every accepted input re-encodes to exactly the same bytes, including after random mutation
  - random truncation is always rejected
  - round trips for `Value`, integers, IDs, base32, Edits, Checks, bundles and claims
  - ECDSA and Ed25519 sign/verify, with flipped bits rejected
  - history queries match a straightforward linear scan

## Deviations and decisions
- **CaptureId layout:** the 5 schema bits are the top bits (56–60) and the 56 random bits are the low bits. The docs give no layout.
- **Example display ID:** "H4IL-7QK2-M9XR-T3VA" can't be produced, because the Crockford alphabet has no I or L. Output always uses the canonical alphabet; I and L are still accepted on input and read as 1.
- **Nested structs have no version key:** only the top-level hashed or exported structs carry key 0 = version. They are `WitnessClaim` (2), `FingerprintBundle`, `KeyInfo`, `Submission` and `LocationOpening` (each 1). Nested maps such as Kind, Edit, Time and Loc start their fields at key 0.
- **Compact encodings I chose** (the docs leave these open):
  - `Kind` is a map `{0: tag, 1: parent, 2: root, 3: edits}`.
  - `Edit` is `{0: kind, 1: params}`, where params are an integer-key map per kind.
  - `dims`, `key`/`counter` and `StatusChange` are 2-element arrays.
  - `Checks` is a single packed uint.
  - The grid is a 384-byte big-endian byte string.
  - PDQ shapes are an array of `[tag, bytes(32)]` pairs.
- **Edit parameters filled in:**
  - `orient` is a code 0–7.
  - `rotate` is limited to ±1000 (`deg_e2`, i.e. ±10°).
  - `sigma` is stored as `sigma_e2` (u16).
  - `recompress.format` uses an `ImageFormat` enum.
- **`active_at` means "trusted":** it returns true for Active or Retired, since retired keys keep signing derivatives. `revoked_after` counts both Suspended and Revoked, matching §4.4. History must be strictly increasing in time, no two consecutive entries may have the same status, and nothing may follow Revoked.
- **Claim rules I enforce:**
  - `coarse` is allowed only on derivatives.
  - A derivative must have `loc.commit = None` ("derivatives carry coarse or nothing").
  - Dimensions must be non-zero.
  - At most 64 edits.
  - `CertSerial` is 1–20 bytes, unsigned, with no leading zero byte.
- **Low-S:** `es256_verify` accepts any S, as standard ECDSA does. `es256_verify_low_s` enforces low-S, and `es256_normalize_low_s` is provided for the signing side. witness-verdict and the registry need to choose which verify function to call.
- **Bitset bit positions** (`LocationReasons`, `GnssChecks`) are my own choices. They are listed in `src/evidence.rs` and pinned by tests.

# ---- crate report ----
`witness-merkle` is finished. `cargo test -p witness-merkle` passes (92 tests), `cargo clippy -p witness-merkle --all-targets -- -D warnings` is clean, and `cargo build -p witness-merkle --target wasm32-unknown-unknown` succeeds. The only dependency is `witness-format`.

**Public API**
- **Hashing** (`hash`)
  - `Hash = [u8;32]` and `ZERO`.
  - `mmr_leaf(data)`, `mmr_node(l,r)`, `mmr_bag(left_peak,right_bag)`, `mmr_root(leaf_count,bag)`, `smt_leaf(key:u128,value)`, `smt_node(l,r)`.
  - Each uses a different one-byte tag (0x00, 0x01, 0x02, 0x03, 0x10, 0x11) through `witness_format::crypto::hash_ds`.
- **MMR** (`mmr`)
  - `trait MmrStore { size() -> u64; get(pos) -> Option<Hash>; push(Hash) }`, with an implementation for `Vec<Hash>`. Nodes are stored in post-order by position, which maps directly onto `StableVec<[u8;32]>`.
  - `Mmr<S>`:
    - `new(store)`: works out the leaf count from the store size and returns `CorruptStore` if the size is not a valid MMR size. Nothing is rebuilt.
    - `leaf_count`, `store`, `into_store`.
    - `append(data) -> index`, `append_leaf(hash) -> index`.
    - `peaks`, `root`, `root_at(n)` (the root at any earlier size).
    - `prove(index)`, `prove_at(index, n)`, `prove_consistency(old, new)`.
  - `InclusionProof { index, leaf_count, path: Vec<Hash> }` with `verify(root, data)`, `verify_leaf(root, leaf_hash)` and `compute_root(leaf)`. The path is: siblings from the bottom up, then the bag of the peaks to the right, then the peaks to the left, nearest first.
  - `ConsistencyProof { old_size, new_size, path }` with `verify(old_root, new_root)` and `roots()`. The path is the old peaks, then the new-only subtree hashes in left-to-right order, then the bag of the new peaks to the right.
  - Both `path` fields are `Vec<Hash>`, so they map straight onto `mmr_proof: vec blob`.
  - `MAX_LEAVES = 2^62`.
- **SMT** (`smt`)
  - `DEPTH = 128`.
  - `trait SmtStore { get(depth,prefix); set; remove }`, with an implementation for `BTreeMap<(u8,u128),Hash>`. `prefix` is the top `depth` bits of the key, matching `StableBTreeMap<(u8,u128),_>`.
  - `key_of(&KeyId) -> u128`, reading the id as big-endian.
  - `empty(height) -> &'static Hash`: the empty-subtree hashes, computed once.
  - `Smt<S>`: `new`, `root`, `leaf(key)`, `insert(key, &value_hash) -> root`, `remove(key) -> root`, `prove(key)`.
  - `SmtProof { bitmap: u128, siblings }`: bit h is set when the sibling at height h is not the empty hash. It has `verify(root, key, Option<&value>)` (`None` checks non-membership), `compute_root`, `to_bytes` / `from_bytes` (16-byte big-endian bitmap followed by 32·popcount bytes) and `HEADER_LEN`.
- **Combined root:** `certified_data(mmr_root, leaf_count, smt_root) = sha256(mmr_root ‖ leaf_count_be8 ‖ smt_root)`.
- **Errors:** `Error { MissingNode(pos), CorruptStore, OutOfRange, Full, Malformed }`.

**Complexity** (the tests check the read and write counts with a counting store)
- **MMR append:** exactly `trailing_ones(n)` reads and `1 + trailing_ones(n)` writes, so O(1) amortised and at most O(log n). All hashes are computed in a stack array before anything is pushed, so a failed read leaves the store untouched.
- **MMR root, inclusion and consistency proofs:** O(log n) reads, and proofs are O(log n) long.
- **Verifiers:** no allocation, and the work is bounded by the proof length, so a hostile size such as `u64::MAX` cannot make them loop or panic.
- **SMT update:** exactly 128 reads, 129 writes and at most 128 hashes. Only non-empty nodes are stored, so removing a key frees its nodes.
- **SMT proof:** exactly 128 reads, and its size is 16 + 32 × (number of non-empty siblings). With 1,000 random keys that was about 10 siblings.

**Tests: 92 in total**
- 81 unit tests next to the code and 11 integration tests in `tests/`.
- **MMR:** roots checked against an independent reference for every size from 0 to 300. Every leaf at every size from 1 to 300 proves and verifies, and proofs with a changed sibling, index, leaf count or root fail.
- **Consistency:** every (old, new) pair up to 96, plus 3,000 random pairs up to 300 with tampering. Also chains of proofs, a forked history being caught, and hostile size and length combinations.
- **SMT:** random insert, update and remove sequences against a naive recomputation. Also membership and non-membership proofs, the extreme keys 0 and `u128::MAX`, neighbouring keys, rejection of a sibling that is actually the empty hash, and random garbage bytes.
- **Speed:** `mmr_exhaustive` takes about 18 s and the unit tests about 6 s in debug builds, because SHA-256 is unoptimised there.

**Deviations from the docs**
1. The crate uses `std` rather than `no_std`, because `std::sync::LazyLock` holds the 129 empty-subtree hashes. `wasm32-unknown-unknown` and canisters both have `std`.
2. `leaf_count` goes into `certified_data` as 8 big-endian bytes. The architecture doc does not say how it is encoded.
3. The MMR root also includes the leaf count, as the task asked, so the leaf count appears twice in `certified_data`.
4. `MmrStore::push` cannot fail. A canister's stable-memory version should trap if growing memory fails, which rolls the whole message back.
5. Because `Vec<Hash>` implements `MmrStore`, calling `vec.get(range)` on a `Vec<Hash>` while `MmrStore` is imported picks the trait method instead. Write `as_slice().get(..)` in that case.
6. The registry can either keep the peaks in a stable cell as the doc says, or recompute them in O(log n) reads with `peaks()`. The crate does not require the cell.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-merkle/`:
- `Cargo.toml`
- `src/lib.rs`, `src/error.rs`, `src/certified.rs`, `src/hash.rs`, `src/testutil.rs`
- `src/mmr/mod.rs`, `src/mmr/pos.rs`, `src/mmr/inclusion.rs`, `src/mmr/consistency.rs`
- `src/smt/mod.rs`, `src/smt/empty.rs`, `src/smt/proof.rs`
- `tests/common/mod.rs`, `tests/mmr_exhaustive.rs`, `tests/smt_model.rs`, `tests/certified.rs`

# ---- crate report ----
`witness-watermark` is done. All three checks pass: `cargo test -p witness-watermark` (110 tests), `cargo clippy -p witness-watermark --all-targets -- -D warnings`, and `cargo build -p witness-watermark --target wasm32-unknown-unknown`. The only dependencies are `witness-format` and `witness-image`, there are no code comments, and I edited nothing outside `crates/witness-watermark/`.

**Payload interop is checked against TrustMark's own code.** I ran Adobe's `bchecc.py` (pure Python, read in full first, run in isolated mode) to produce six golden packets for chosen capture ids. Our encoder matches all six bit for bit. It also matches the packets in their Rust crate's test suite, including the rejected non-canonical packet. The generator polynomial comes out as `0xC_A760_24D7` (degree 35, over GF(2^7) with field polynomial 137), and the tests check it.

**Public API**
- **Payload** (`payload.rs`, `bch.rs`):
  - `encode(CaptureId) -> [bool; 100]` builds the 100-bit packet: 61 data bits (capture id, most significant first), then 35 BCH bits, then version `0001`.
  - `decode(&bits, &erased) -> Option<Decoded>`, where `Decoded { capture_id, errors, erasures }`.
  - `soft_decide(&logits, dead_zone) -> HardDecision { bits, erased }`: a bit is 1 when its logit is above 0. It is erased when the absolute logit is not above the dead zone, and always when the logit is NaN.
  - `decode_logits` does both steps at once. `payload_tensor` gives the encoder's 0.0/1.0 input.
  - Constants: `PAYLOAD_BITS`, `DATA_BITS`, `ECC_BITS`, `VERSION_BITS`, `MAX_CORRECTABLE` = 5, `ERASURE_BUDGET` = 10, `DEFAULT_DEAD_ZONE` = 0.25.
- **Preprocessing** (`resize.rs`, `tensor.rs`):
  - `resize_pil_bilinear(view, w, h)` reproduces Pillow's `Image.BILINEAR` exactly, including its downscale smoothing, 22-bit fixed-point weights and horizontal-then-vertical rounding.
  - `model_input(view, placement, gray, &mut [f32])` writes the 3×256×256 tensor scaled to [-1, 1].
  - Also exported: `normalize`, `MODEL_SIZE`, `PLANE_LEN`, `TENSOR_LEN`.
- **Geometry** (`geometry.rs`):
  - `processing_region(w, h)` takes the centre square when one side is more than twice the other.
  - `hypotheses(w, h, sensor) -> Hypotheses` returns at most 4 placements: the copy as-is, then centred, start-anchored and end-anchored inside a canvas with the sensor's aspect ratio.
  - `Placement`, `Rect`, `mean_gray`, `SENSOR_ASPECT` = (4, 3), `MAX_HYPOTHESES`.
- **Residual** (`residual.rs`):
  - `Residual::from_model(stego, &cover, strength)` computes strength × (stego − cover), clamped to ±0.2, with NaN becoming 0.
  - `Residual::apply(&mut Rgb8)` upsamples with PyTorch's bilinear convention, adds to each pixel, clamps and rounds.
  - Constants: `DEFAULT_STRENGTH` = 1.0, `RESIDUAL_LIMIT` = 0.2.
- **Port** (`watermarker.rs`):
  - `Watermarker` has two required methods, `encode` and `decode`, which work on tensors. These are what witness-android's ONNX sessions will implement.
  - It provides `embed`, `detect` and `read` (which tries the geometry hypotheses) on top of them, plus a `strength()` that implementations can override.
  - `NullWatermarker` returns `WatermarkError::Unavailable` immediately from every method.
- **Errors:** `WatermarkError { Unavailable, Inference, Image(ImageError) }`.

**Complexity**
- **BCH:** parity is 8 table lookups. Decoding is at most 96 × 5 steps for syndromes, then Berlekamp–Massey, then a root search over 99 positions. With erasures it runs exactly two fill-in attempts, valid while 2 × errors + erasures ≤ 10.
- **Resize:** O(input pixels). It streams rows through a small ring buffer (about 25 KB for a 12 MP image) instead of holding a full intermediate image.
- **Residual apply:** O(w·h) time with O(w) extra memory: two cached rows plus the column weights. There is never a full-frame float buffer, and padding outside the centre crop is computed on the fly.
- **Geometry:** padded canvases are never built in memory. When the copy covers the whole region, rows are read straight from the source without copying.

**Tests: 110** (104 unit, 6 integration in `tests/pipeline.rs`):
- **Payload:** golden vectors, and 0–5 random errors always recovered with the exact count. With 6 to 25 errors the decoder never returns the original; over 95% of those cases are rejected outright.
- **Erasures and soft decisions:** errors combined with erasures within budget, errors on the never-sent padding bits rejected, schema bits, dead zone, NaN and infinite logits.
- **Resize:** hand-derived Pillow weights and outputs; streaming result identical to a straightforward full-buffer implementation.
- **Residual:** matches an independent per-pixel PyTorch-style reference exactly; output bounded to ±26 levels; deterministic.
- **Integration:** a fake block-pattern model round-trips through embed and detect. A centred crop is recovered by the second hypothesis after the as-is reading fails, a left-anchored crop is recovered, and reads stop after at most 4 decoder calls.

**Deviations from the brief and docs**
1. **Decoder resolution is 256, not 245.** Python's PyTorch decoder for the Q model uses 245, but the ONNX `decoder_Q` used by TrustMark's Rust and JS code takes 256. We ship the ONNX model, so 256 is right for us.
2. **The residual follows TrustMark's Rust crate, not the Python.** That means the ±0.2 clamp, no per-channel mean subtraction, no feathered edges, and the same boundary handling for very wide or tall images. ARCHITECTURE §5 says we vendor the Rust crate's logic. The upsample uses PyTorch's formula; the Rust crate's own resize filter is the same maths for upscaling.
3. **Pixels are rounded, not truncated.** Python truncates the final value, which darkens every pixel by half a level on average; the Rust crate rounds.
4. **`decode` reports errors and erasures separately** instead of one "corrected" count. For the verdict's "≤ 2 BCH-corrected errors" rule, it is safer to treat errors + erasures as the count.
5. **Only the last two version bits are checked,** as TrustMark's Python does (erased bits count as a match). Only the BCH_5 schema is supported.
6. **Hypothesis orientation is a heuristic.** The sensor aspect is turned to match the copy (landscape when width ≥ height), and padding is a single mean gray level.
7. **`detect` and `read` take an `RgbView`, not `&Rgb8`,** so crops need no copy; callers pass `img.view()`.

The default strength (1.0) and dead zone (0.25) are tuning knobs for the M0 spike; neither has been calibrated against real model output.

Files are in /Users/andrejones/Desktop/workspace/projects/witness/crates/witness-watermark:
- Cargo.toml
- src/lib.rs
- src/bch.rs
- src/payload.rs
- src/resize.rs
- src/tensor.rs
- src/geometry.rs
- src/residual.rs
- src/watermarker.rs
- src/error.rs
- src/testutil.rs
- tests/pipeline.rs

# ---- crate report ----
witness-attest report

Status: `crates/witness-attest` is done, and I edited nothing outside it. These all pass with the `dev-attestation` feature both off and on:
- `cargo test -p witness-attest`: 162 tests (151 unit, 8 against the real root certificates in `tests/real_roots.rs`, 3 on the status list in `tests/status_list.rs`).
- `cargo clippy -p witness-attest --all-targets -- -D warnings`: clean.
- `cargo build -p witness-attest --target wasm32-unknown-unknown`: builds in debug and release.

The crate is `no_std` + `alloc`. Its wasm32 dependency tree has no getrandom, wasm-bindgen, js-sys or tokio. I also broke the code deliberately eight times (placement rule, signature check, validity exemption, future-dated challenge, Late wall-clock check, boot state, patch level), and each time a test failed.

**Public API**
- **Top level:** `verify(&Enrollment, &Policy, &Context) -> Result<Attested, Error>` runs every §4.2 rule. Cheap checks run first and signatures last.
  - `Enrollment { chain: &[impl AsRef<[u8]>] (leaf first), key_id: Option<KeyId>, late: Option<LateOpening { install_nonce, wall_ms }> }`.
  - `Context { now_ns, challenge_secret: &[u8;32], revocations: &RevocationList }`.
- **`Policy`:** holds anchors, package, min_version, release_digests, min_patch, fresh_ttl_ns (1 h), late_ttl_ns (7 d), max_skew_ns (10 min) and active_ns (60 d).
  - `Policy::witness(release_digests)` is a const fn that builds it with `ANCHORS`, `app.witness`, min_patch 202501 and min_version 1.
- **`Attested`:** key_id, pubkey (leaf SPKI), security, provisioning, freshness, patch, boot, device_locked, created_ms, attestation and KeyMint versions, chain_serials (normalized hex), and quota_key (SHA-256 of `chain[1]`). `Attested::key_info(enrolled_ns, active_ns)` returns a `witness_format::KeyInfo`.
- **Helpers in `policy`:**
  - `attestation_index`: finds the KeyDescription closest to the root. It must be in the leaf, and directly below the provisioning-info certificate if that extension is present. A second occurrence higher up is rejected.
  - `provisioning`: classifies the chain as Rkp, Factory or Dev.
- **`challenge`:**
  - `issue_fresh(secret, now_ns) -> [u8;40]` produces `issued_ns_be ‖ HMAC-SHA256`.
  - `fresh_issued_ns` checks the HMAC in constant time.
  - `late_challenge(nonce, wall_ms)` is `SHA-256("witness-offline" ‖ nonce ‖ wall_ms_be)`.
- **`x509::Certificate::parse`:** a zero-copy view with tbs, sig_alg, signature, serial, issuer, subject, validity, spki, basicConstraints, keyCertSign, the KeyDescription bytes and the provisioning-info flag.
  - Other methods: `public_key`, `verify_signed_by`, `valid_at`, `is_ca`.
- **`pubkey`:** `PublicKey` (P256, P384, Rsa) has `from_spki` and `verify(SigAlg, msg, sig)`. `SigAlg` covers RSA PKCS#1 v1.5 and ECDSA, each with SHA-256, -384 or -512.
- **`chain`:** `check_length` (3 to 8 certificates), `root_kind`, `check_issuers` (CA flag, pathLen, keyCertSign), `check_validity`, `verify_signatures`.
- **`keydesc`:**
  - `KeyDescription::parse`, `AuthList` (purpose, algorithm, keySize, ecCurve, creationDateTime, origin, rootOfTrust, os/vendor/boot patch levels, attestationApplicationId), `RootOfTrust`, and `AppId::parse` (package names, versions and signing digests).
  - Duplicate known tags are rejected; unknown tags are skipped.
- **`roots`:** pinned SPKIs `GOOGLE_RSA_SPKI` and `GOOGLE_EC_CA1_SPKI`, plus the AOSP emulator EC and RSA roots. `ANCHORS` includes the AOSP roots only with `dev-attestation`. Also `TrustAnchor`, `RootKind` and `find`.
- **`serial::Serial`:** a fixed 32-byte big-endian value, so ordering is numeric. Constructors are `from_be_bytes`, `from_hex` (any case, leading zeros allowed) and `from_decimal`; `to_hex` gives lowercase with no leading zeros.
- **`revocation`:** `RevocationList::new` sorts and deduplicates, keeping the most severe status. Lookup is `get`/`status`, and `to_cbor`/`from_cbor` give canonical bytes that must be strictly sorted. Also `RevocationStatus` (with `key_status()`) and `RevocationReason`.
- **`status_json::parse_status_list(json) -> RevocationList`** uses serde_json.
- **Smaller modules:** `der` is a strict zero-copy TLV reader, `time` parses UTCTime/GeneralizedTime into Unix seconds, and `Error` is one fine-grained enum.

**Complexity**
- Parsing is O(chain bytes) with no copies; the only allocation is one `Vec` of n ≤ 8 certificates. Each issuer key is parsed once (n−1 times in total), and signatures are checked last.
- A revocation lookup is a binary search over 34-byte entries, O(log m) with no allocation per lookup.
- Building the list is O(m log m). It is about 120 KB for the current list.
- Extension and authorization-list duplicate checks use bitmasks, so they are O(1) per field.

**Deviations from the docs, and why**
1. **The real Google status list mixes decimal and hex serials.** I fetched it today: 991 of the 1,787 keys are 17–20 digit decimal 64-bit serials, and the rest are 32-character hex. Any all-digit key is therefore stored under both readings (one false positive in about 2^64), otherwise roughly half the revocations would never match.
2. **I hand-wrote the DER parser instead of using the `der` crate.** der 0.7 can't decode tag numbers above 30, and the KeyDescription uses [701], [704], [706] and [709].
3. **Root certificate validity and the root self-signature are never checked.** Trust comes from the pinned key. The 2016 `f92009e853b6b045` root certificate expired in May 2026 and would otherwise break chains that still include it.
   - Issuer validity is exempt for Factory and Dev chains and enforced for Rkp chains. The leaf is always enforced.
4. **Issuer and subject names are not compared.** The signature is the cryptographic link, and byte-level name comparison risks rejecting real devices.
5. **The KeyDescription must be in the leaf.** This is stricter than Google's guidance and stops a chain where an attested key is used to sign some other key. Witness doesn't use app attest keys.
6. **Late challenge details:**
   - The Enrollment must carry `{install_nonce (1–64 bytes), wall_ms}` so the registry can recompute the hash.
   - Both creation time and wall_ms must fall within [now − 7 d, now + 10 min skew].
   - Creation time comes from the hardware list, then the software list, then the leaf's notBefore.
   - A Fresh challenge dated in the future is rejected.
7. **Dev relaxations depend on the root, not the feature.** They apply only when the chain ends at a Dev anchor: Software security level is allowed, boot state and lock are not enforced, and enforced fields may come from the software list. The feature only decides whether `ANCHORS` contains the AOSP roots.
   - Challenge, app identity, patch level and key properties are still enforced for Dev chains. Local builds therefore need the debug signing digest in their `Policy`.
8. **Status list edge cases:** an unknown status is treated as SUSPENDED (fail closed), an unknown reason as UNSPECIFIED, and unparseable keys are skipped.
9. **The challenge HMAC has no domain prefix,** matching §4.1. The key is a canister-held `[u8;32]`.
10. **Format and parsing choices:**
    - A certificate serial may be negative or non-minimal and is read as unsigned bytes. More than 32 significant bytes is rejected.
    - Unknown critical extensions are rejected, as PKIX validators do.
    - Extra trailing fields in a KeyDescription are accepted for forward compatibility.
    - RSA keys must be at least 1024 bits; the AOSP emulator roots are 1024-bit.

**For the registry owner:** fill in the real release signing digest(s) and confirm `DEFAULT_MIN_PATCH` before shipping. These chain shapes still need checking against real Pixel and Samsung chains in M0.

Files are in /Users/andrejones/Desktop/workspace/projects/witness/crates/witness-attest:
- Cargo.toml
- src/ (lib, der, time, oid, serial, pubkey, x509, roots, chain, keydesc, challenge, revocation, status_json, policy, error, plus a tests.rs beside each larger module)
- roots/*.spki.der (pinned keys, loaded with `include_bytes!`)
- tests/common/mod.rs (shared DER and certificate builder)
- tests/real_roots.rs
- tests/status_list.rs
- tests/data/*.der (the real Google root certificates and the AOSP software-attestation certificates and keys)

# ---- crate report ----
Final report for `witness-fingerprint` (crate path: /Users/andrejones/Desktop/workspace/projects/witness/crates/witness-fingerprint).

All three checks pass:
- `cargo test -p witness-fingerprint`: 122 tests pass (118 unit tests and 4 integration tests).
- `cargo clippy -p witness-fingerprint --all-targets -- -D warnings` is clean.
- `cargo build -p witness-fingerprint --target wasm32-unknown-unknown` succeeds.

The only normal dependencies are `witness-image` and `witness-format`. getrandom, wasm-bindgen, tokio, reqwest, ort and c2pa are not in the wasm32 dependency tree. The only dev-dependency is `pdqhash = "=0.1.1"`, used as the test oracle. There are no code comments. Library code uses integers only: no floats, so no `mul_add` question, and the output is bit-identical on aarch64 and wasm32.

## Public API

**Hash** (`hash.rs`)
- `PdqHash([u8;32])`: uses the same byte and bit layout as the reference hex output.
- Methods: `words`, `from_words`, `bit`, `set_bit`, `count_ones`, `distance` (popcount over 4×u64), `to_hex`, `from_hex`, `Display`.
- Constants: `HASH_BITS`, `HASH_BYTES`.

**PDQ** (`pdq.rs`, `dct.rs`, `dihedral.rs`)
- `pdq(LumaView) -> Option<Pdq>`: computes the hash and quality. Returns `None` below 5 px.
- `pdq_dihedral(LumaView) -> Option<PdqDihedral>`: returns all 8 dihedral hashes from one DCT.
- `Pdq { hash, quality }` with `indexable()` (quality ≥ 50). `PdqDihedral { hashes, quality }` with `get(Dihedral)` and `pdq()`.
- `Dihedral`: Identity, Rotate90 (clockwise), Rotate180, Rotate270, MirrorHorizontal, MirrorVertical, Transpose, AntiTranspose. Each variant has `apply` and `index`.
- Constants: `MIN_HASHABLE_DIM`, `MIN_INDEX_QUALITY`, `MAX_QUALITY`.

**Shapes** (`shape.rs`)
- `Aspect`: the 7 aspects. `Anchor`: Start, Center, End.
- `ShapeTag(u8)`: the tag is `aspect<<2 | anchor`, so FULL = 1 and the largest is 26. Methods: `new`, `from_u8` (validates), `aspect`, `anchor`, `placement`, `rect`.
- `stored_shape_tags(w,h)`: gives 19 tags for a 4:3 landscape and 16 for 3:4 portrait.
- `fit_long_edge`: scales dimensions to a target long edge.
- `working_luma(RgbView)`: area-resizes to a 512 px long edge, then converts to luma.
- `stored_shapes(LumaView) -> Vec<ShapeHash>`: `ShapeHash` has `to_format()`, which returns a `PdqShape`.
- `query_hashes(RgbView, &[Rect]) -> Vec<QueryHash>`: per rect it gives Base, Mirror (from the dihedral), Inset and Outset. Duplicate hashes are dropped and the total is capped at 16.

**Grid** (`grid/`)
- `Grid`: cells are `[u16;192]` stored as `stable<<8 | signs`, plus a portrait flag. Methods: `from_bundle`, `pack`, `signs`, `stable`, `dims`.
- `grid(LumaView)`: aspect-preserving resize to 256 px. Each cell gets 8×8 sub-block means, a const-table 3×3 DCT with DC dropped, signs against the median of the 8 coefficients, and stability bits.
- `compare(a,b) -> Option<GridDiff>`: `GridDiff` holds a per-cell distance (0..255), a compared mask, `agreement_q16`, `compared_fraction_q16`, and bit counts.
- `hypotheses(w,h)`: stored shapes × scales {90, 100, 110}.
- `align(&Grid, original_dims, copy: LumaView) -> Option<Alignment>`: returns the best and runner-up scores, the diff for the best, and an `accepted` flag.
- Other exports: `fingerprint(RgbView) -> Result<FingerprintBundle, FingerprintError>` (the capture-side bundle), `is_portrait`, `grid_dims`.

**MIH** (`mih/`)
- `PERMUTATION`: a const 256-entry permutation built at compile time by a `const fn` (SplitMix64 plus Fisher–Yates, `PERMUTATION_SEED = 0x5749544E45535331`). A test freezes its values.
- `permute`, `chunks` (16×u16), `index_keys` (16), `probe_keys` (272).
- `BucketKey { chunk, value }`: `packed()` gives a u32. `stable_key(sub)` gives the §7.3 layout `chunk<<32 | value<<16 | sub`, and `from_stable_key` reverses it.
- `Posting { local_ix, shape }`: 5-byte `encode`/`decode`.
- `trait PostingStore { insert; visit(key, impl FnMut(Posting)) }`: the storage port. `insert(store, hash, posting)` writes one hash into its 16 buckets.
- `Prober::probe(store, query, limit) -> Ranked { candidates, truncated }`: candidates are ranked by matched-chunk count, ties broken by lowest `local_ix`. `truncated` is set only when the cut-off falls on a tie.
- Constant: `MIH_RADIUS = 31`.

## Complexity
- **PDQ**: O(W·H) time for the reused streaming Jarosz filter. Memory is O(64·H) for the Jarosz buffers plus a fixed 4096-entry buffer. The integer DCT folds the input on its cosine symmetry, which halves the multiplies to about 41k per hash. The median uses `select_nth_unstable`, which is O(256).
- **Stored shapes**: about 19 runs on a working image of at most 512 px; roughly 11 ms per capture in release.
- **Grid**: one summed-area table, then O(1) per sub-block sum, so about 64 lookups per cell. Each alignment hypothesis is about 192×(64+~100) operations. The full alignment of about 57 hypotheses takes roughly 2 ms in release.
- **MIH probe**: 272 bucket visits. Cost is O(postings) for marking plus O(h log h) for ranking. The bitset is cleared per chunk by reusing the list of touched entries, and it is reused across queries. The permutation step is 256 bit operations.

## Tests (122)
- **PDQ agreement with `pdqhash`**:
  - On 64×64 grayscale inputs at least 55 of 60 cases match exactly, and none differ by more than 2 bits.
  - On 40 random-size synthetic colour scenes the mean distance is at most 4 bits, the maximum is at most 12, and quality is within 8.
  - Hashes of transformed images are also checked against the oracle.
- **Dihedral**: all 8 variants equal the hash of the actually transformed 64×64 image exactly, and group relations hold. On non-square images the gap is at most 32 bits, with a mean of at most 20.
- **Robustness**: JPEG at quality 90/70/40 stays within 12/18/28 bits. Recompress plus downscale stays within 24. Unrelated images are at least 60 bits apart.
- **MIH**: the pigeonhole guarantee holds against 200 adversarial distance-31 spreads (2 flipped bits in each of 15 chunks plus 1 in the last). A distance-32 spread is shown to be missed, confirming 31 is the exact radius. A brute-force property test confirms every stored pair within 31 is returned. Dedup, ranking, truncation ties, reuse and large indices are also covered.
- **Grid**: brightness ±25, contrast ×1.3 and ×0.6, and gamma all keep agreement at or above 0.93. A pasted patch is detected and localised with at most 3 false cells. Flat paint-over is detected. Hypothesis alignment finds social crops, rejects unrelated images, and reports the patch inside a cropped copy.
- **Integration (`tests/pipeline.rs`)**: capture → bundle → MIH index → query from a platform crop, a mirror and an edit → alignment.

## Deviations and decisions
1. **Integer PDQ instead of float.** Luma is BT.601 rounded to u8, the Jarosz filter is witness-image's fixed-point Q8 version, and the DCT table is i32 cosines scaled by 2^16. The table was generated offline and embedded as a const, and a test re-derives it from `cos`. Differences from the float reference come from luma and Jarosz rounding: a mean of 4 bits or less on colour scenes and exact matches on 64×64 grayscale. The PDQ DCT uses frequencies 1..16 with no DC, as in Meta's code. Quality follows the C++ integer formula `trunc(Δ·100/255)`, summed, divided by 90, capped at 100.
2. **Inset/outset is ±4% of each dimension (2% per side).** Measured PDQ drift is about 20 bits per 1% per side, so 4% per side would leave gaps the base hash and the inset/outset hashes cannot cover.
3. **Shapes are hashed on a 512 px working image**, and query rects are scaled into it, so stored and query hashes go through the same path. The stored set skips an aspect equal to the frame. It adds start and end anchors only when the slack is more than 5% of the frame; otherwise it keeps center only.
4. **Grid design choices** (the docs leave these open): the 8 bits come from the 3×3 low-frequency DCT minus DC. The median is the mean of the two middle coefficients. A bit is stable when |d| > ¼·mean|d| and its amplitude is at least 1 luma level. Comparison uses the union of stable bits from both grids, so texture pasted into flat areas is also caught. Cells outside the copy are masked through a presence mask. For alignment, the copy is sampled at 512 px.
5. **Alignment acceptance**: τ is 0.75 agreement, the margin is 0.10, and at least 48 cells must be compared. The runner-up must be geometrically distinct, meaning shifted by more than half a cell or resized by more than 5%. Without that rule, neighbouring placements such as 4:5 end and 3:4 end compete with the true placement. These thresholds are constants and need tuning against the benchmark.
6. **MIH dedup**: a per-chunk bitset gives distinct-chunk counting, and then a sort and run-length pass aggregates counts. The bitset grows lazily to the largest `local_ix` seen, which is fine for trusted shard data. `fingerprint()`'s portrait flag comes from the original dimensions, so rounding in the working image cannot flip it.

## Suggestions for the shared contract
None are needed. `witness-format`'s `FingerprintBundle` holds a single quality value, so per-shape quality, which the "≤49 not indexed" rule needs, is only available at runtime through `ShapeHash.pdq.quality`. If the index has to enforce that rule per shape after storage, the bundle would need a quality field per shape.

# ---- crate report ----
witness-c2pa (c2pa_lite) is built and every gate passes. `cargo test -p witness-c2pa` runs 176 tests (168 unit, 8 integration against c2pa-rs) with no failures. `cargo clippy --all-targets -D warnings` is clean and `cargo build --target wasm32-unknown-unknown` succeeds. The wasm32 build pulls in none of getrandom/js, wasm-bindgen, tokio, reqwest, ort or c2pa, and the code has no comments.

**c2pa-rs interop (M0 gates, all passing)**
- c2pa-rs reads manifests we write with no failure codes and the right active label, and it sees `app.witness.claim`.
- When the Witness Ed25519 root is set as c2pa-rs's trust anchor, it reports our chain as **Trusted**. So the ARCHITECTURE §4.3 gate holds: c2pa-rs signs with a Witness CA cert, and `Reader` reports no `signingCredential.invalid`.
- Our `data_hash` equals c2pa-rs's hard binding on files c2pa-rs signed.
- c2pa-rs also flags pixel tampering in our files as a dataHash failure.
- A c2pa-rs manifest without `app.witness.claim` comes back as `Other` (other credentials).
- A c2pa-rs manifest with the Witness claim as a *created* assertion verifies in c2pa_lite as Witness with the binding intact. The same claim as a *gathered* assertion is rejected.

**Public API** (all in `crates/witness-c2pa/src/`)
- `data_hash(jpeg) -> [u8;32]`: SHA-256 of every byte except exactly the C2PA APP11 segments. Offsets declared in the manifest are never used.
- `seal_jpeg(jpeg, &ManifestSpec, &dyn Signer, &TrustAnchor) -> Sealed`: removes any existing C2PA store and inserts the new one after the leading APP0/APP1 segments. Returns the new file, the data hash, the C2PA claim CBOR and the COSE_Sign1.
  - `ManifestSpec` holds the id, generator, actions and the Witness claim CBOR.
  - `Signer` has `sign(&[u8]) -> Vec<u8>` (DER or raw signature accepted) and `cert_chain()`.
- `verify_jpeg(jpeg, &TrustAnchor, at: Option<u64>) -> Report`
  - `Report` carries `data_hash` and `credentials`, which is `Absent`, `Other` or `Witness(WitnessManifest)`. `binding_intact()` compares the data hash with both the manifest's declared hash and the claim's `asset_hash`.
  - A broken Witness manifest returns `Err`, which the caller treats as "unverified credentials".
- `verify_seal(witness_claim_cbor, c2pa_claim_cbor, cose, anchor, at) -> Seal`: the registry path, with no JPEG needed. It rebuilds the assertion box and checks it against the claim's hash list.
- `TrustAnchor::from_der(root)` loads the root, and `x509::verify_chain` / `verify_leaf` check the device certificate against it.
- Certificate builder: `cert::root_tbs` / `leaf_tbs` produce the bytes to sign and `assemble(tbs, sig)` wraps them. `build_root` / `build_leaf` take a signing closure.
- Lower-level modules are public too: `jpeg` (segment reader), `app11` (`locate`, `write_packets`), `jumbf` (box parse/build including bfdb/bidb), `claim`, `assertions`, `cose` and `manifest`.

**Complexity**
- One O(n) pass over the file; entropy-coded data is only scanned for 0xFF.
- A store that fits in one segment is read without copying; a multi-segment store gets one allocation of exactly its size.
- Hashing streams over the kept byte ranges without copying them.
- Sealing makes one output allocation of the exact size and calls the signer exactly once. The exclusion length is solved by a monotone fixed-point loop that settles in a few rounds.
- Duplicate checks sort first (O(k log k)) and assertion lookup is a binary search. A crafted header or certificate can't force quadratic work.

**Tests** cover:
- round trips at segment-chunk boundaries and with random chunk sizes;
- out-of-order, missing or duplicated segments, mismatched repeated headers, and a second manifest store;
- malformed JUMBF, CBOR, COSE, claim and certificates, including every byte flip and truncation of the device cert;
- tampered pixels (binding broken), tampered manifest (signature or hash failure), wrong anchor and validity windows;
- seeded random-mutation loops;
- a flip of every byte inside the C2PA segments of a sealed file. Any manifest still accepted after a flip must have unchanged signed content.

**Where I departed from the docs**
1. `seal_jpeg` also takes the `TrustAnchor`. It refuses to seal if the chain doesn't lead to the Witness root, or if the claim's `asset_hash`, `key_id` or `cert_serial` don't match the file and the signing cert. A bad seal fails at capture instead of at verification.
2. `verify_jpeg` and `verify_seal` also require the claim's `key_id` and `cert_serial` to match the signing certificate.
3. The certificate builder has a two-step form (bytes to sign, then `assemble`) as well as the closure form. The registry's `sign_with_schnorr` call is async, so a synchronous signer trait can't wrap it.
4. The subject key identifier is the first 20 bytes of SHA-256 (RFC 7093 allows this), so no SHA-1 dependency.
5. Only created assertions count as attested. Redacted assertions and v1 claims are rejected when a Witness claim is present.
6. A leaf longer than 20 bytes with the high bit set can't be issued: the x509-cert crate rejects it.
7. Sealing replaces any existing C2PA store instead of keeping it as an ingredient.

**Notes**
- The interop tests need the `c2pa` dev-dependency (host only, `rust_native_crypto`, no OpenSSL), plus `c2pa_cbor` and `jpeg-encoder`.
- Adding these dependencies updated the shared `Cargo.lock`.
- The flip-every-C2PA-byte test takes about 10 s in a debug build.

Everything is in `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-c2pa/`. The shared test helpers are in `src/testkit.rs` and the c2pa-rs tests are in `tests/c2pa_rs.rs`.

# ---- crate report ----
witness-match is built at `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-match`. All three gates pass:
- `cargo test -p witness-match` passes 214 tests.
- `cargo clippy -p witness-match --all-targets -- -D warnings` is clean.
- `cargo build -p witness-match --target wasm32-unknown-unknown` succeeds in both dev and release.

The crate's only dependency is witness-image, so its wasm tree has no getrandom, wasm-bindgen or tokio. There are no code comments. I used a separate target directory in the scratchpad so I wouldn't block other agents on cargo's lock. The only file outside the crate that changes is the shared `Cargo.lock`, because of the new witness-image dependency.

A seeded check of 30 random scenes (each cropped, scaled, flipped every other time, filtered, vignetted and re-saved as JPEG at quality 70) gave 0 false flags on the unedited copies, and caught the pasted edit in all 30.

## Public API
- **Geometry**
  - `Rect`: an integer rectangle with intersect, union, clamp, expand and contains.
  - `Similarity { a, b, tx, ty, mirrored }`: a 4-parameter transform with `apply`, `inverse`, `scale` and `mirror_x(width)`. It maps reference coordinates to query coordinates, using continuous pixel coordinates where pixel centres sit at +0.5.
- **Randomness**
  - `Rng` trait with `next_u64` and `below`, plus `SplitMix64` as the injectable deterministic generator.
- **Features**
  - `detect_features(luma, full_w, full_h, &FeatureParams) -> Features`: FAST-9 corners with a score and non-max suppression, over a pyramid that shrinks by 0.8 per level. Each corner gets an orientation from its intensity centroid, using a Q14 integer trig table (32 bins, no float trig).
  - Descriptors are rotated BRIEF, 256 bits, from a pattern generated at compile time.
  - `hamming` compares two `Descriptor` values.
- **Alignment**
  - `align(query, reference, &AlignParams, &mut impl Rng) -> Option<Alignment { transform, inliers, matches, confidence }>`.
  - It shrinks both images so the long edge is at most 512 px, matches with a ratio test, then runs RANSAC on a 2-point similarity.
  - The mirrored hypothesis comes from re-detecting features on the horizontally flipped query.
  - It refines with least squares on the inliers. `confidence` = inliers / matches, and it returns `None` below 10 inliers.
- **Extraction**
  - `extract_photo_regions(screenshot, &ExtractParams) -> Vec<Candidate { rect, confidence, kind, overlay_mask }>`.
  - `kind` is one of `Natural` (up to 3), `FullMinusBars` or `LargestBand`; exact duplicates are removed.
  - `OverlayMask` has `covers(x, y)` and `relative_to(x, y)`, which moves the mask into the coordinates of a cropped query.
- **Diff**
  - `diff(query, reference, &transform, overlay, declared_blurs, &DiffParams) -> PixelDiff { map, blurs, assessment }`.
  - Steps: bilinear warp onto the reference, per-channel 16-knot tone curve (pool-adjacent-violators), 4×4 gain field, windowed change map, then classification.
  - `high_frequency_energy` and `blur_holds` are exported for the blur check.
- **Classification** (generic over any change map, so the grid path can reuse it)
  - `ChangeMap` is a trait; `StateMap` and `CellState` are its concrete form; `GridShape` is 16×12 or 12×16.
  - `components()` finds 8-connected regions with run-length union-find.
  - `classify(map, grid, explained, &ClassifyParams) -> Assessment { compared_fraction, changed_fraction, explained, classification }`.
  - `Classification` is `CouldNotCompare` (changed fraction above 0.35, or nothing compared), `Consistent`, `Inconclusive(TooManyComponents | InBorderBand | NotRinged)`, or `Local(Vec<Component>)`.

## Complexity
- FAST runs in linear time per pyramid level and keeps only a 3-row buffer; the pyramid as a whole is still linear.
- Orientation and descriptors cost O(keypoints × patch size).
- Descriptor matching is brute force: at most 600 × 600 popcounts per hypothesis.
- RANSAC costs iterations × matches; the refinement is linear in the inliers.
- Warp, photometric fit (3 passes, gain fit sampled every 2nd pixel, a 16×16 solve), change map and labelling are all linear in reference pixels.
  - The change map keeps running column sums, so its extra memory is O(width).
  - Labelling memory is O(number of runs).
- The ring check is bounded by 6 components.
- Extraction is linear, with integral images giving constant-time rectangle counts.

## Tests (214)
- **155 unit tests:** reference vectors, comparisons against simple reimplementations (components vs flood fill, window stats, box sums, integral images), and seeded property loops.
- **Alignment (15):** identity, down- and up-scaling, centre and offset crops, flips, flipped crops, 90° rotation, JPEG and filtered copies, unrelated images, determinism, degenerate inputs.
- **Diff (29):** JPEG, filter, vignette and scaled/cropped/flipped copies are not flagged. Pasted edits are found with the right region, including through crop, scale, flip and JPEG. Also covers more than 6 edits, the border band, more than 35% changed, tiny edits, all the declared-blur cases, overlay masking and the window radius.
- **Extraction (12):** feed in light and dark, stories, carousel, X timeline, WhatsApp viewer, TikTok side rail, fallbacks, determinism, 1×1 and strip-shaped inputs.
- **Pipeline (3):** screenshot → extract → align → diff, end to end.

## Deviations from the docs, and why
1. **Change test.** The pure SSIM structure term σxy/(σxσy) cannot see texture being removed (a flat window scores 1). I use the contrast-structure term, which drops in that case, computed on luma, plus a luma window-mean test (20 levels) and a per-channel colour-mean test (40 levels). Per-channel structure on its own flagged 22% of a plain JPEG copy because of chroma subsampling.
2. **No full integral images for SSIM.** Sliding column sums give the same linear time with O(width) memory instead of five full-frame tables.
3. **Robust photometric fit.** It starts from per-bin medians, then alternates tone and gain fits 3 times, discarding pixels more than 40 levels off. A plain mean fit let one pasted edit skew the curve and flag the whole frame.
4. **Edge margin.** Query pixels within max(2 px, 1.5% of the query's long edge) of its border are not compared. Without this, JPEG ringing and UI bleed at the candidate edge caused border-band "Inconclusive" results on clean screenshots.
5. **Extraction.** Work size is a long edge of 512, so a phone screenshot is about 236 px wide. Rectangles are rounded inward and inset by 1 working pixel. Overlay masks use 4 px blocks: blocks that are partly flat with sharp edges, grown into neighbouring fully flat blocks, then widened by one block. Thin story progress bars are not reliably masked at this size; they sit in the border band anyway.
6. **Refinement.** Lucas–Kanade is replaced by iterated least squares on the inliers, as the task specified.
7. **Border band** is measured from the bounding box of the compared pixels rather than the full frame, so crops work. A verified declared-blur rectangle explains a component only if it contains it after growing by the window radius + 1.
8. **Blur check.** The mean |Laplacian| inside the rectangle must be at most max(3.0, 0.35 × the reference's value). This works whether the reference is the unredacted original or the redacted version.
9. **Own types.** `Rect` and `Similarity` are defined in this crate, since witness-format is not a dependency.

## Notes for whoever wires up `judge` (I didn't edit any shared contract)
- Map `Local` to an ALTERED candidate, `CouldNotCompare` to "couldn't compare", and `Consistent` / `Inconclusive` to MATCHES_WITH_CHANGES.
- **Risk:** a flat, graphic-looking edit inside a photo can be masked as an overlay and so never compared. `judge` should look at how much of the frame was masked (the masked block count, or a drop in `compared_fraction`).

# ---- crate report ----
Both crates are built: `judge()` in witness-verdict and the witness-core facade with the `seal()` pipeline. `cargo test`, `cargo clippy --all-targets -D warnings` (also with `--features testkit`) and the wasm32 build (dev and release) all pass for both crates. There are 95 tests in total, and the code has no comments. I edited only `crates/witness-verdict` and `crates/witness-core`. The witness-core wasm32 tree contains none of getrandom, wasm-bindgen, tokio, reqwest, ort or the c2pa crate.

## witness-verdict public API
- **`judge(input, &oracle, &Options) -> Verdict`**: runs ARCHITECTURE §6.5 in order: Witness credentials, then exact hash, then watermark and PDQ candidates, then expanding roots to their public derivatives, then top-k confirmation, then classification.
- **`trait Oracle`** (synchronous):
  - `get_key`, `get_record`
  - `exact(asset_hash, Option<pixel_hash>)`
  - `by_watermark` (≤32, earliest first)
  - `pdq_probe(&[PdqHash]) -> PdqProbe{hits, truncated}`
  - `derivatives(root)`, `reference_image` (encoded bytes, decoded inside core so results stay deterministic), `fingerprints`
  - `index_pending()` (defaults to false)
- **`Record`** fields: `record_ix`, `record_id`, `claim`, `anchored_ns`, `flags`. The registry-derived late-anchor and clock-mismatch flags arrive in `flags`.
- **`Options`**: `android()` (k=8) and `web()` (k=3), `new(anchor, k)`, `with_watermarker`, `at(time)` for injected certificate-validity time, and `max_pixels`.
- **`Verdict`** fields: `version` (`VERDICT_V`=1), `outcome`, `flags`, `reasons`, `declared` edits, `regions`, and `reference`. `reasons` is a bitset that always lists in the same order.
- **`Reference`** fields: `record_id`, `claim`, an optional anchor, `evidence` (Seal / ExactHash / Pixels / Grid), and confidence, compared and changed values in Q16.
- **`Region`**: Q16 coordinates normalized to the reference frame.
- **Tuning constants**: `TAU_PIXEL_Q16` and `MIN_COMPARED_Q16` (both 0.3), `FULL_COMPARED_Q16` (0.75), `GRID_CHANGED_DISTANCE` (128), the foreign-watermark rule (≤2 corrected errors and every PDQ distance > 90), and the caps `MAX_*`.
- **Feature `testkit`** (off by default): `MemoryOracle`, `TestCa`/`TestDevice` (a Signer, plus claim and seal helpers), `FixedWatermarker`, and `photo()`.

## witness-core public API
- **Modules**: `witness_core::{format, image, fingerprint, matching, c2pa, merkle, attest, watermark, verdict}`, plus `judge`, `Options`, `Oracle` and `Verdict` at the top level.
- **`prepare(&Capture) -> Prepared`**: decode, then orient, then embed the watermark through the `Watermarker` (if that fails it seals without one and records `watermark: None`), then encode. It then decodes the final JPEG and computes `pixel_hash`, fingerprints and `asset_hash` from it.
- **`Prepared::claim(ClaimTemplate)`**: builds and validates the claim, and rejects a watermark id that differs from the root capture id.
- **`finish(...)`** and **`seal(...)`**: `seal` is `prepare` followed by `finish`. They produce a `SealedCapture` holding the JPEG, claim, `record_id`, the `Submission` for the registry, the fingerprints and a `watermarked` flag. The split lets Android assign the counter between `prepare` and signing. `SealError` covers the failure cases.

## Complexity
- **Seal path**: one parse of the C2PA data in the file plus O(1) oracle reads.
- **Exact path**: O(n) over at most 16 hits.
- **Image work**: one decode per input, scaled down to 2048 px on the long edge. Query hashing reads at most 4 rectangles and returns ≤16 hashes. Watermark reading tries at most 2 views × 4 geometry hypotheses.
- **Candidates**: wm ≤32 + PDQ ≤32 + derivatives, total capped at 64 and sorted by record index to remove duplicates (O(n log n)). Each candidate's distance is ≤16×19 popcounts.
- **Confirmation**: costs at most k × (align + diff), and the grid fallback only rescores 192 cells.
- **Determinism**: no HashMap anywhere, and the RANSAC seed comes from `record_ix`.

## Tests (95)
- **witness-verdict: 72.** Every branch you listed is covered with fake-oracle tests:
  - original; presentation-only derivative ⇒ ORIGINAL; content-edited derivative ⇒ MATCHES_WITH_CHANGES (declared)
  - seal broken, both anchored and unanchored
  - SEALED_UNANCHORED; a revoked key while unanchored
  - unenrolled key, a key whose public key mismatches, a foreign CA, and non-Witness C2PA, all ignored
  - key revoked after anchor (flag); key not trusted at anchor time (falls through and skips the exact path)
  - exact hits on asset hash, pixel hash, a derivative, the earliest anchor, and an oracle returning wrong records
  - watermark transplant warning versus a hint when the watermark is noisy; unconfirmed PDQ hint
  - couldn't-compare; local edit ⇒ ALTERED with regions, via pixels and via the grid
  - no-reference cap; truncated candidates; top-k; choosing the derivative that best explains the pixels; index pending
  - seeded loops: no false accusations, local edits caught, random mutations never forge an Original, random bytes never panic
- **witness-core: 10 unit tests and 13 end-to-end tests.** Each end-to-end test seals a synthetic photo with a test signer, then judges the original, a copy with credentials stripped, recompressed, downscaled and cropped copies, a local edit, a copy with no reference, a tampered file, an unanchored rotated capture, sealed derivatives, and a foreign photo carrying the watermark. Every case is checked under both the Android and web options, and both must give the same outcome.

## Deviations from the docs
- **Oracle is synchronous.** Web callers will need to fetch the data first or wrap the calls.
- **Ranking of the best candidate.** "Changed area" counts only the area of significant changed regions (a consistent copy counts as 0). The raw changed fraction is dominated by JPEG noise, and ranking on it picked the wrong record. Ties then prefer the comparison that needed fewer declared-blur explanations.
- **Region coordinates.** Regions are normalized to the reference frame (the reference pixels, or the 16×12 cells on the grid path), not to the copy's coordinates.
- **Extra cases for `COULDNT_COMPARE`.** It is also raised when less than 75% of the frame was compared (heavy crops) and when the classification is inconclusive.
- **Unanchored seals.** SEALED_UNANCHORED requires the key to be trusted now. Otherwise the reason is `KeyNotTrusted` and judging continues to the later steps.
- **Seal-broken detail.** An ALTERED verdict for a broken seal still includes the record as its reference.
- **Exact-hash hits.** They are rechecked against the hashes, so an oracle can't inject unrelated records, and they also carry the revoked-after-anchor and dev-key flags.
- **Inputs our decoder can't read.** They still get the exact asset-hash lookup. For non-JPEG files, `asset_hash` is the SHA-256 of the whole file.
- **Pixel hash.** It is computed from the decode before orientation is applied, to match how `seal()` computes it.
- **No `c2pa_lite` alias.** BUILD.md calls the C2PA module `witness-core::c2pa_lite`, but I followed the task and exposed it as `c2pa` only.

Main files:
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-verdict/src/{judge,credentials,exact,query,candidates,confirm,oracle,options,verdict,testkit,tests}.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-core/src/{lib,seal}.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-core/tests/e2e.rs`

# ---- crate report ----
All six QA tasks are done. The full workspace test suite passes, clippy and `fmt` are clean, the dependency check passes, and the golden digest is identical on aarch64 and wasm32 (run on both).

**Results**
1. **Tests:** `cargo test --workspace` passes with 1,298 tests and 0 failures. Nothing needed fixing.
2. **Lint and format:** `cargo clippy --workspace --all-targets -- -D warnings` is clean. `cargo fmt --all` changed formatting only in `crates/witness-image/src/buffer.rs` and `crates/witness-image/src/decode.rs`.
3. **Wasm build:** `cargo build -p witness-core --target wasm32-unknown-unknown --profile wasm` succeeds. The new `/Users/andrejones/Desktop/workspace/projects/witness/scripts/check-core-deps.sh` reads the normal-edge wasm32 tree of `witness-core` with features shown. It fails if it finds `getrandom` with the `js` or `wasm_js` feature, or any of `wasm-bindgen`, `tokio`, `reqwest`, `ort` or `c2pa` (the exact crate name, so `witness-c2pa` is not flagged). The tree is clean today and contains no `getrandom` at all.
4. **Golden test:** `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-core/tests/golden.rs`
   - It generates 50 synthetic images (16–255 px per side) from a SplitMix64 seed. For each one it hashes the JPEG bytes, the pixel hash of the source and of the decoded image, the grid cells, the PDQ quality, all 8 rotated/flipped PDQ hashes, the MIH index and probe keys, and the full `fingerprint()` output.
   - It then seals 2 originals into the in-memory test oracle and runs `judge()` on 8 inputs with two option sets, Android and web: sealed, recompressed, cropped, an unrelated photo, and garbage bytes.
   - It checks the expected list of 16 outcomes, then compares the SHA-256 of everything with the committed digest `07ac345d7c835f301824b10ed4706fc0e9f11a3137ab553a336bf60e55da7708`.
   - It runs as a normal native test, and on wasm32 through `wasm-bindgen-test` under node. `/Users/andrejones/Desktop/workspace/projects/witness/.cargo/config.toml` sets `wasm-bindgen-test-runner` as the wasm32 runner, and `wasm-bindgen-test = "=0.3.80"` is a wasm32-only dev-dependency in `crates/witness-core/Cargo.toml`.
   - Both runs pass: about 5.0 s native and 5.5 s on wasm32.
5. **Comments and dead code:** the core crates have no `//` or `/* */` comments. Clippy finds no dead code in library code. The six `#![allow(dead_code)]` lines are on shared test-helper modules that several test binaries include, and the attest one is also used by the attest crate's unit tests. Every helper they flag is used somewhere, so I kept them.

**Tests per crate**

| Crate | Tests | Split |
|---|---|---|
| witness-attest | 162 | 151 unit + 11 integration |
| witness-c2pa | 176 | 168 + 8, including the c2pa-rs `Reader` check |
| witness-core | 24 | 10 + 13 end-to-end + 1 golden |
| witness-fingerprint | 122 | 118 + 4 |
| witness-format | 140 | 135 + 5 |
| witness-image | 186 | 143 + 43 |
| witness-match | 214 | 155 + 59 |
| witness-merkle | 92 | 81 + 11 |
| witness-verdict | 72 | 72 unit |
| witness-watermark | 110 | 104 + 6 |
| **Total** | **1,298** | |

**Wall time:** 141 s for a warm run in debug; the first run, including compiling, took about 164 s. The slowest parts are fingerprint `pipeline` (26 s), fingerprint unit tests (23 s), merkle `mmr_exhaustive` (16 s) and c2pa unit tests (14 s).

**Open issues**
- **Copies not compared:** at 192×144, a plain recompressed copy of a sealed photo gets `MatchesWithChanges` with the "couldn't compare" flag set. I would expect a full comparison there, so the verdict/match owners should look at it.
- **Wasm test coverage:** only `golden.rs` runs on wasm32. The other tests use plain `#[test]` and would need `cfg_attr` like `golden.rs` to run there.
- **Runner version:** `wasm-bindgen-test-runner` must stay at 0.2.130 to match `wasm-bindgen` in the lockfile.
- **Possible speed-up:** adding `[profile.dev.package."*"] opt-level = 2` in the root `Cargo.toml` would likely cut suite time a lot. I left the root profile alone.
- **Nothing committed:** everything under `crates/`, plus `Cargo.toml` and `Cargo.lock`, is still untracked in git.