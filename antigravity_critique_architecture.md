# Architecture Review & Critique: Witness (`docs/ARCHITECTURE.md`)

**Reviewed document:** [ARCHITECTURE.md](file:///Users/andrejones/Desktop/workspace/projects/witness/docs/ARCHITECTURE.md) (690 lines, status: pre-build, revision 2)  
**Cross-checked against:** [SPEC.md](file:///Users/andrejones/Desktop/workspace/projects/witness/docs/SPEC.md), [antigravity_critique_spec.md](file:///Users/andrejones/Desktop/workspace/projects/witness/antigravity_critique_spec.md), [antigravity_critique_design.md](file:///Users/andrejones/Desktop/workspace/projects/witness/antigravity_critique_design.md), and the current workspace layout (`canisters/`, `client/`, `crates/`).

---

## Executive Summary

Revision 2 of `ARCHITECTURE.md` is a **major engineering upgrade** over the original spec. Decisions D1–D14 directly address almost every architectural flaw from the first review: introducing the Witness C2PA CA on threshold Ed25519 (§4.3), separating `Fresh` vs. `Late` key enrollment (§4.1), moving `pixel_hash` post-encode inside a single pinned decoder (§3.2, §6), replacing global-only PDQ with multi-shape + TrustMark + alignment confirmation (§6.1–6.5), replacing composite queries with client fan-out (§7.3), persisting MMR/certification inputs (§7.2, §7.4), and encrypting case links with `#fragment` keys + AAD (§9).

However, **several concrete cryptographic, ICP-scaling, and state-machine bugs remain** that will cause wrong verdicts, broken builds, or DoS/upgrade failures if left unfixed going into M0/M1:

1. **`judge()` verdict state-machine bugs (§6.5, §3.2, §7.3):** Signed **Derivatives** (cropped/blurred posts) and stripped Derivative `pixel_hash` hits both return **`ORIGINAL`** (green), while unanchored captures from **already-enrolled** keys fall through to `NO_RECORD` instead of `SEALED_UNANCHORED`.
2. **Split-brain C2PA parsing vs. D1 determinism (§2, §6.5, §8.2):** `witness-core` excludes `c2pa-rs`, and web uses `@contentauth/c2pa-web` in JS, yet `judge()` claims to parse C2PA manifests and produce 100% bit-identical results across `aarch64` and `wasm32`. JUMBF `APP11` exclusion validation is also unspecified.
3. **Key Attestation quota & validity bugs (§4.2, §7.2):** Limiting RKP **intermediates** to `2 keys / 30 days` and `20,000 submits / day` ignores that RKP intermediates are shared across large OEM/cohort fleets—potentially bricking the app for legitimate users—while a global `FACTORY` token bucket lets one leaked keybox DoS all legacy devices.
4. **Impossible `KeyInfo` "certified MMR keyed by `key_id`" (§7.2):** An MMR is append-only by integer index, not keyed; it cannot prove current non-revocation of a `key_id`. Meanwhile, `KeyInfo` stores only one `{status, status_since}` pair, and `registry.enroll` lacks an async re-entrancy guard around `sign_with_schnorr`.
5. **Canister scaling & upgrade contradictions (§7.3–7.5, §7.7):** `pages` rebuilds `HttpCertificationTree` in `post_upgrade` (directly violating §7.1's no-rebuild rule) and stores unnecessary per-`case_id` shells; `index` uses a broken `u32` key layout for overflow sub-keys and faces >2B–5B instruction risk on 1M-capture MIH `StableVec` reads; `/v/{case_id}#k=...` URLs carry no `media-N` shard routing info.
6. **Browser CSP / data-drain / user-deletion gaps (§7.6, §8.1, §8.3, §10.3):** `case.witness.app`'s strict hash/SRI CSP blocks WebAssembly (`'wasm-unsafe-eval'`) and transitive dynamic `import()`s; idle prefetching of the 47–62 MB watermark decoder on `/p/` views drains mobile 4G data; `@Generable` per-template classes contradict adding templates as data; and user-triggered deletion / Witness Plus entitlement verification on ICP are unarchitected.

---

## 1. What Revision 2 Gets Right (Strengths)

- **Trust-minimized C2PA identity (§4.3):** Having `registry` act as the Witness CA via threshold Schnorr Ed25519 cleanly solves `c2pa-rs` rejecting Android Key Attestation leaf certs (missing EKU/AKI) and prevents expiring RKP batch certs from invalidating old signatures.
- **Single-signature capture path (§3.2, §5):** Making the C2PA `COSE_Sign1` the Witness seal halves StrongBox signing latency and eliminates redundant signing state.
- **Honest privacy boundary (§3.4, §7, §11):** Keeping the 256-bit location salt off C2PA/chain, making perceptual data (`index`, `pages`, `media`) erasable while keeping only commitments in `registry`, and using fresh anonymous principals per key are well thought out.
- **Pragmatic ICP query model (D10, §7.3):** Avoiding ICP composite queries (which share one 5B instruction budget and cannot be replicated) in favor of client-side fan-out over ordinary queries with replicated update fallback is the right ICP architecture.
- **Adversarial watermark-transplant rule (§6.5):** Returning `NO_RECORD` with a warning and **hiding the reference image** when a watermark hits but alignment fails closes the framing/attribution attack cleanly.

---

## 2. Critical Security & Cryptographic Bugs

### 2.1 `judge()` Returns `ORIGINAL` for Derivatives and Drops Enrolled Unanchored Captures (§6.5, §3.2, §7.3)
Tracing the pseudocode in `judge(input, oracle)` ([ARCHITECTURE.md §6.5](file:///Users/andrejones/Desktop/workspace/projects/witness/docs/ARCHITECTURE.md#L317-L349)) reveals four verdict bugs:

1. **Signed Derivatives return `ORIGINAL` (lines 321–325):**
   ```text
   if c2pa active manifest has app.witness.claim:
       ...
       return file_data_hash == claim.asset_hash ? ORIGINAL : ALTERED(seal_broken)
   ```
   A `Derivative` (e.g., a photo cropped, filtered, and blurred in Post Composer, §3.2 line 147) has its own `WitnessClaim` and C2PA signature. When checked, `file_data_hash == claim.asset_hash` is true, so `judge()` returns **`ORIGINAL`** instead of `Matches an original, with changes` (or `Original · N declared edits`), contradicting `SPEC.md §4.8` and §9.
2. **Stripped Derivatives return `ORIGINAL` via `oracle.exact` (line 328, §7.3 line 406):**
   Because `media` uploads require a `Derivative` record (§7.5 line 450), *every* posted preview/derivative has its `asset_hash` and `pixel_hash` indexed in `index`. If someone downloads a cropped/blurred public post (stripped of C2PA by Instagram/X) or a `/r/{record_id}.jpg` preview and checks it, `oracle.exact(pixel_hash(img))` hits the `Derivative`'s `pixel_hash` and returns **`ORIGINAL`**!
   - **Fix:** `exact` hits must check `rec.claim.kind`: only `Original` records produce `ORIGINAL`; `Derivative` exact hits must resolve to `MATCHES_WITH_CHANGES` with declared edits shown against the root (or redacted derivative) reference. Also note: `pixel_hash[..16] -> record_ix` (singular, line 406) breaks on identical-pixel frames (e.g., lens cap / solid black); store a small bounded list or disambiguate.
3. **Unanchored captures from an *enrolled* key fall through to `NO_RECORD` (lines 322–327):**
   Suppose a user enrolled their key weeks ago, enters airplane mode, shoots a photo, and checks it locally (or shares the local C2PA file before outbox upload completes). `rec = oracle.get(record_id(claim))` is `None`. Line 326 only checks *"if claim valid but key not yet enrolled: return SEALED_UNANCHORED"*. Because the key *is* enrolled (and captures before enrollment don't even have a C2PA manifest yet per §4.1 line 176!), line 326 is dead/inverted logic, and an enrolled unanchored capture falls through to `NO_RECORD`!
   - **Fix:** Check `oracle.get_key(claim.key_id)` + `cose_verify` + `file_data_hash == claim.asset_hash` even when `rec` is `None`. If signature + hash are valid and `key` is enrolled (or claim-only pre-enrollment on local device), return `SEALED_UNANCHORED`; if `file_data_hash != claim.asset_hash`, return `ALTERED(seal_broken)`!
   - Also clarify how `SEALED_UNANCHORED` maps to the **4×4 verdict confusion matrix** (§12 line 604), since `SPEC.md §4.8` only defines 4 verdicts.
4. **`key_revoked_after_anchor` is dropped by `judge()` (§4.4 vs. §6.5 line 323):**
   Line 323 requires `key.status ∈ {Active, Retired} at rec.anchored`, returning `ORIGINAL` silently with no `key_revoked_after_anchor` flag (promised in §4.4 line 216). Furthermore, `KeyInfo` only stores a single `{status, status_since}` (§7.2 line 366), which cannot represent state transitions if a `SUSPENDED` key is later reinstated or subsequently `REVOKED`.

### 2.2 Split-Brain C2PA Verification vs. D1 Determinism & `c2pa.hash.data` Exclusions (§2, §3.2, §6.5, §8.2)
- `witness-core` explicitly forbids the `c2pa` crate (§2 line 117), and the website relies on `@contentauth/c2pa-web` in JS (`c2pa.mjs`, §8.2 line 516).
- Yet `judge()` in `witness-core` (§6.5 lines 321–325) parses the C2PA active manifest, verifies `COSE_Sign1`, verifies the cert chain to Witness CA, and compares `file_data_hash == claim.asset_hash`—and §6 line 281 requires **100% bit-identical `judge()` outputs** on `aarch64` vs. `wasm32`.
- **Two risks here:**
  1. If `witness-core` implements `c2pa_lite` (JUMBF box extraction + COSE_Sign1 + X.509 Witness CA chain verification + JPEG `APP11` exclusion hashing), **why is `@contentauth/c2pa-web` needed at all** (adding JS bundle weight and pre-1.0 risk)? Conversely, if `c2pa-rs` (Android) and `c2pa-web` (Browser) extract the manifest outside `witness-core` before calling `judge()`, malformed JUMBF / multi-manifest / ingredient edge cases can diverge between platforms, breaking D1.
  2. **Exclusion-range spoofing:** §5 computes `asset_hash` *before* `c2pa::Builder::sign` runs (line 235 vs. 238), and §3.2 line 148 defines `asset_hash` as *"the sealed JPEG with C2PA segments excluded"*. In `witness-core`, `file_data_hash` must strictly exclude **only** valid C2PA JUMBF `APP11` (`0xFFEB` with `JP` identifier) segments and hash *every other byte* of the JPEG—never trusting arbitrary `c2pa.hash.data` exclusion offsets from an untrusted manifest! Specify the exact JUMBF `APP11` extraction and `c2pa_lite` ownership in `witness-core`.

### 2.3 Key Attestation Quota, Validity & Async Re-entrancy Flaws (§4.2, §7.2)
1. **RKP Intermediate Quota Will Brick Legitimate Users (§4.2.8, §7.2.4):**
   Under Google's Remote Key Provisioning (RKP), the intermediate CA certificates signed by Google's root are **shared across millions of devices** (by OEM/SoC/RKP server cohort), and even leaf-signing batch keys can be refreshed or shared depending on implementation. Enforcing `≤ 2 keys per 30 days per RKP intermediate` (§4.2 line 196) and `≤ 20,000 submits per RKP intermediate per day` (§7.2 line 387) will reject legitimate users once two devices under the same RKP intermediate enroll in a month.
   - **Fix:** If throttling by RKP batch/device lineage is intended, clarify *which* certificate in the chain is keyed (leaf-signing batch cert vs. intermediate CA) and verify against real Pixel/Samsung RKP chains in M0. Even per batch-cert, 2 keys / 30 days breaks users who reinstall twice or run both TEE + StrongBox keys.
2. **Global `FACTORY` Token Bucket = Instant DoS (§4.2.8):**
   A single leaked `FACTORY` keybox (acknowledged in §13) can exhaust the global `FACTORY` token bucket and block **every** legacy non-RKP Android phone worldwide from enrolling. Rate-limit per `chain_serials` / `key_id` + Proof-of-Work or progressive backoff, not a shared global bucket.
3. **Blanket Validity Bypass Under Legacy RSA Root (§4.2.1 line 186):**
   *"Enforce validity except under the legacy RSA factory hierarchy, which Google says to trust regardless of validity."* Google's guidance applies to expired **factory batch/intermediate** certificates; bypassing validity checks on the **leaf** certificate undermines §4.1.2's `notBefore ≤ 7 days` fallback check for `Late` offline keys when `CREATION_DATETIME` is absent. Enforce leaf validity (`notBefore`/`CREATION_DATETIME`) even on `FACTORY` chains.
4. **Async Re-entrancy in `registry.enroll` (§4.2, §4.3, §7.2):**
   `registry.enroll` calls ICP's management canister `sign_with_schnorr` (an inter-canister `await`). In ICP Rust canisters, state mutations must reserve quotas and lock `key_id` **before** `.await` (with rollback on error), or concurrent `enroll` calls can bypass quotas and duplicate-key checks across the suspension point.
5. **Key Retirement Deadlock (§4.1.4, §7.2.2):**
   §4.1 says a key retires for new originals after 30 days, *and* *"keeps working past 30 days until a fresh one can be enrolled."* How does `registry.submit` distinguish a legitimate offline phone past 30 days from a malicious client that simply never retires its key? If `registry` uses a hard `enrolled_at + 30d` cutoff, offline captures taken on day 31 before reconnecting fail `submit`; if `registry` only marks a key `Retired` when the client voluntarily stops using it, the 30-day rotation policy isn't server-enforced.

### 2.4 `KeyInfo` "Certified MMR Keyed by `key_id`" Contradiction (§7.2 line 392)
Line 392 states: *"`KeyInfo` gets its own small certified MMR keyed by `key_id`, with versioned status entries."*
- An MMR is an **append-only positional list** (`0..N-1`), not a key-value map. You can prove a leaf existed at some index in an MMR, but you **cannot prove that a newer `Revoked` version for `key_id` was not appended later** without scanning to the tip!
- **Fix:** Store current `KeyInfo` in a certified **`RbTree` / Sparse Merkle Tree keyed by `key_id`** (giving `O(log K)` proof of current status for `keyinfo_root`), and keep an append-only MMR/log only if historical state transitions need an audit trail. Also align `KeyInfo` fields between §4.2.9 (which includes `chain_serials`) and §7.2 Candid (which omits `chain_serials`).

---

## 3. ICP Canister & Scaling Flaws

### 3.1 `pages` `post_upgrade` Contradiction & `/v/*` Bloat (§7.1, §7.4, §7.7)
- **Direct contradiction:** §7.1 line 358 mandates *"No structure is rebuilt in `post_upgrade`"*, yet §7.4 line 441 states `pages`'s `post_upgrade` **rebuilds `HttpCertificationTree` from persisted hashes**. `HttpCertificationTree` (`ic-http-certification`) lives in the **4 GiB wasm32 heap** and performs `O(N log N)` SHA-256 node re-hashes on insert. At hundreds of thousands or millions of `/p/{record_id}` pages, `post_upgrade` will hit ICP's upgrade instruction ceiling or heap OOM.
- **Unnecessary `/v/{case_id}` entries:** §7.4 line 438 says `/v/{case_id}` is a **"Generic `noindex` shell"**. Storing a separate certified tree entry per `case_id` wastes memory/instructions and leaks `case_id` existence—use **one** wildcard certified asset for `/v/*` (since `WebRouter` reads `location.pathname` and fetches ciphertext from `media-N`).
- **No sharding path for `pages`:** Unlike `index-N` and `media-N`, `pages` is singular because `witness.app` maps `TXT _canister-id` to one canister (§7.7). Define an explicit entry ceiling / eviction / redirect-to-archive strategy for `pages` before `HttpCertificationTree` outgrows one canister.

### 3.2 `index-N` Key Layout, MIH Instruction Budget & Erasure Races (§7.3)
1. **Broken `u32` key layout for overflow buckets (line 404):**
   `StableBTreeMap<u32 (chunk_pos<<16 | value), Blob≤4KiB>` with *"overflow sub-keys for hot buckets"*. `chunk_pos` (4 bits) + `value` (16 bits) already occupy bits `0..19`. Putting an overflow index `sub_ix` in bits `20..31` destroys key locality for B-tree range scans (`sub_ix` becomes the MSB, scattering overflow pages across the entire tree). Either use a `u64` / `(u8, u16, u16)` key or pack `(chunk_pos << 28) | (value << 12) | sub_ix` so overflow blocks stay contiguous.
2. **1M-capture MIH instruction budget likely exceeds 2B–5B (lines 411–414):**
   At 1M captures × ~17 PDQ shapes = 17M postings across 65,536 buckets/chunk, average bucket depth is ~259 postings. For 16 query hashes × 272 buckets/hash = 4,352 B-tree lookups and **~1.13M candidate posting checks**—each triggering a `StableVec` read (unless cached/deduplicated first) via `ic0.stable64_read`! Even before hot buckets (which are never truncated), 1.13M stable syscalls will likely breach the 2B target (and even ICP's 5B query cap).
   - **Recommendation:** Deduplicate `(local_ix, shape)` hits in a small hash/bit-set across the 16 chunks *before* reading full 32-byte hashes from `StableVec`, pack all `S` shapes of a capture into one contiguous `StableVec` row (`local_ix`) to cut random reads, and run the 1M-record `canbench` gate early (M2, not M5) to confirm realistic shard capacity (likely 250k–500k captures/shard rather than 1M).
3. **`capture_id` / `by_watermark` flooding (§3.1, §6.5, §7.2, §7.3):**
   `capture_id` is only 56 random bits and `registry.submit` does not restrict who claims a `capture_id`. An attacker can submit 2,000 fake records carrying a victim's public `capture_id`, bloating `by_capture` (`vec text`) and `index`'s `Watermark` list (`probe` has no cap on returned watermark hits). Cap `Watermark` hits per `capture_id` or rank/filter them carefully.
4. **Incomplete `erase()` and timer races (§7.3):**
   - `registry`'s 5s timer calls `index.put` asynchronously without mentioning a `draining: bool` re-entrancy guard (if `index.put` takes >5s, overlapping timer ticks can race or stall).
   - `erase(record_ix, reason)` only erases one `index-N` shard; there is no coordinator workflow ensuring `registry` doesn't re-drain an in-flight `put` for an erased record, nor removing derivative links across shards.

### 3.3 `media-N` Shard Routing Missing from Case URLs (§7.5, §7.7, §9)
- Case links are `https://case.witness.app/v/{case_id}#k={b64url(k)}` (§9 line 537), while case chunks live on `/c/{case_id}/{ix}` under `m{N}.witness.app` (§7.5 line 449), and `media-N` is **"sharded by count"** (line 444).
- **Nothing in the URL or `registry` tells the recipient's browser which `media-N` shard holds `{case_id}`!**
- **Fix:** Either encode the shard index into `case_id` (e.g., `m0-...` or base32 prefix), or maintain a certified `media` shard map / route lookup. Also note that provisioning a new `m{N}.witness.app` subdomain for every media shard requires manual/API DNS + ACME registration (§7.7), which adds operational friction and Let's Encrypt rate-limit exposure if shards rotate frequently.

### 3.4 Off-Chain Git Publication Clarification (§7.2 line 394)
Line 394 says *"Every hour, `(leaf_count, root)` is published to a public git repository"*. Clarify that an **off-chain publisher job** polls `registry` and commits to git (the canister itself holds no git/SSH secrets).

---

## 4. Matching, Watermark & Determinism Risks (`witness-core`)

1. **Derivatives Required for Public Previews — Even on Unedited Posts (§7.5 line 450):**
   §7.5 requires uploads to reference a `Derivative` record. That means even a user who accepts zero blurs and crops nothing must still mint a `Derivative` record (`edits = []`) to host a `/p/{record_id}` preview—resulting in *two* `record_id`s (Original + Derivative) for one post. Spell out this lifecycle explicitly and ensure `judge()` handles it without misclassifying the Derivative as `ORIGINAL` (§2.1 above).
2. **Float Determinism (`fmadd` Contraction) Across `aarch64` and `wasm32` (§6 line 279):**
   Restricting `witness-core` to `+ − × / sqrt` without SIMD is **not sufficient** for bit-identical floats in Rust: LLVM on `aarch64` can fuse `a * b + c` into hardware `fmadd` (single rounding), whereas `wasm32` (MVP) performs separate `mul` + `add` (double rounding), causing subtle divergence in DCT/ORB/SSIM/isotonic math. Either disable FMA contraction in Rust/target flags, use explicit fixed-point/integer arithmetic where possible, or validate with strict `f32`/`f64` IEEE-754 operations tested in the M2 golden CI gate.
3. **CPU Budget for Alignment/Diff & Multi-Hypothesis Watermark on Mobile Web (§6.4–6.5, §8.2–8.3):**
   - Running ORB + RANSAC + Lucas-Kanade + 16-knot isotonic regression + 4×4 gain field + 8–11 px SSIM at ≥ 1080 px across `top-k = 8` candidates in **single-threaded scalar `wasm32` / `js(IR)`** (especially in pre-iOS 18.2 JS fallback mode!) is compute-heavy and unbudgeted in M0.
   - Similarly, `decode_wm(rects, geometry hypotheses)` (§6.5 line 330) runs a 47.4 MB CNN (`decoder_Q.onnx`) in single-threaded `ort-web` CPU mode (§8.2 line 517). If `geometry hypotheses` tries even 6–12 crops/scales across 3 rects, inference alone will exceed the 10-second mobile budget. Bound the exact number of watermark inferences and measure `wasmJs` + `js` alignment/diff latency in M2.
4. **Missing `Edit` Schema & Browser HEIC/AVIF Fallback Breaking D1 (§3.2, §6 line 280):**
   - `WitnessClaim.kind.Derivative.edits: [Edit]` (§3.2) has no defined CBOR schema, nor does §6.5 define how crop/filter/blur parameters in `Edit` suppress false-`ALTERED` flags without letting an attacker hide edits inside a declared blur's low-frequency mask.
   - §6 line 280 allows browser-decoded HEIC/AVIF fallback, which applies OS/browser color management and breaks D1's bit-identical pixel guarantee (and leaves Android's `Share → Witness` without a HEIC path unless `ImageDecoder` is explicitly bridged with an explicit caveat).

---

## 5. Web, Android & Product-Architecture Gaps

### 5.1 `case.witness.app` CSP Breaks WebAssembly & Dynamic Imports (§8.1 lines 503–504)
- Line 504 states `case.witness.app` uses a stricter CSP: *"Scripts are allowed by hash or SRI from a versioned path only"*.
  1. Without `'wasm-unsafe-eval'` in `script-src`, browsers **refuse to compile `composeApp.wasm`, `witness-web`, and `ort-web`**!
  2. External `<script src="https://app.witness.app/boot.mjs">` with SRI only validates `boot.mjs`; subsequent dynamic `import()` calls inside `boot.mjs` (the Compose wasm/js loader and `witness-bridges`) fail a hash-only CSP unless an `<script type="importmap">` with SRI integrity hashes for every chunk (or `'strict-dynamic'`) is baked into the `pages` shell—creating tight release coupling between `web` and `pages` canisters.
  3. Line 503 omits `img-src` (which must allow `https://*.witness.app` for `/r/` previews and `blob:` for decrypted case images and dropped check files).
- **Reload hazard on `history.replaceState` (line 504):** Dropping `#k=...` immediately after reading the key means refreshing the browser tab (or mobile browser tab eviction) permanently loses the decryption key. Decide whether to keep `k` in memory-only (warn user not to reload) or `sessionStorage` for the active tab.

### 5.2 Mobile Data Drain from Idle Prefetch (§8.3 line 528)
Stage 4 states the **47 MB model + 15 MB `ort-web` (~62 MB)** is *"prefetched when idle unless `saveData` is set"*. Most mobile browsers never set `Navigator.connection.saveData = true` on 4G/5G cellular connections. Prefetching 62 MB silently when a visitor merely opens a `/p/{record_id}` proof link will burn user cellular data and spike `web` canister egress cycles. **Only load/prefetch the watermark decoder on `/check` (or when `connection.effectiveType` / user action warrants it)**, and prioritize the `≤ 12 MB int8` decoder evaluation in M0/M2.

### 5.3 Android Capture Memory, Counter Atomicity & `@Generable` Coupling (§5, §10.1, §10.3)
- **50 MP OOM Risk (§5 line 252):** On Pixel 8/9/10 and modern mid-range phones, full-sensor JPEG decode to `RGB8` (e.g., 50 MP = ~150 MB raw RGB8 × 2 workers = 300 MB + turbojpeg + 17.3 MB ONNX) can trigger OOM. Specify a CameraX `ResolutionSelector` cap (e.g., 12 MP / 4096 px long edge) and explicitly state whether CameraX EXIF / UltraHDR gainmaps are stripped.
- **Counter persistence ordering (§10.1 line 556):** Storing `counter` in a separate app-private file outside the Room transaction (line 240) can lose sync on crash; either persist `counter` inside Room (allocated before signing, committed atomically or incremented via monotonic file+fsync before signing) so a crash never reuses a `(key_id, counter)`.
- **`@Generable` contradicts data-driven A2UI (§10.3 vs. `SPEC.md §4.6`):** Requiring one compile-time `@Generable` Kotlin class + R8 rule per template (line 567, 572) means **new situations cannot be added as data without an app update** (`SPEC.md §4.6` line 105). If only 3 MVP situations exist, either drop the "add situations without an app update" claim in `SPEC.md`, or use a single generic structured schema (`templateId` + typed slot list) rather than per-template `@Generable` classes.

### 5.4 Missing Architectures for MVP Compliance & Monetization (`SPEC.md §10, §11.2, §12.1`)
1. **User Account / Content Deletion vs. Anonymous Principals (`SPEC.md §12.1`):**
   `SPEC.md §12.1` promises account and hosted-media deletion in-app and on the web, yet §4.1 uses *"a fresh anonymous ICP principal"* per key, and `pages`/`media`/`index` expose **no user-callable deletion endpoints** (only controller/moderation principals). A user who wants to delete their own preview/page (especially after reinstalling or from the web) has no authenticated path to do so. Issue a per-post/per-key deletion capability token (or sign a deletion request with the device key / Internet Identity) so users can erase their own erasable data without moderation intervention.
2. **Witness Plus Entitlements & 25 GB Media Hosting (`SPEC.md §10, §11.2`):**
   `SPEC.md` puts Witness Plus (Play Billing via RevenueCat + **25 GB full-res hosting**) in MVP, but `ARCHITECTURE.md` has no design for how a RevenueCat purchase grants quota on `media-N` canisters, how full-res media (>16 MB/case, non-expiring while subscribed) is stored/renewed, or how Internet Identity ties in.

---

## 6. Workspace Layout & `SPEC.md` Sync Discrepancies

- **Workspace scaffolding vs. §2:** `client/native/witness`, `client/androidBenchmark`, and `canisters/media` listed in §2 (lines 86, 102, 108) do not yet exist in the directory tree (`canisters/` has `index`, `pages`, `registry`, `web`; `client/` has `androidApp`, `build-logic`, `core`, `feature`, `webApp`).
- **`SPEC.md` sync drift (violates line 3's *"fix one of them in the same change"*):**
  - `SPEC.md §11.1` line 291 describes *"Batched into one Merkle root roughly every minute"*, whereas `ARCHITECTURE.md §7.2` anchors synchronously per `submit()`.
  - `SPEC.md §4.3` line 74 promises **on-device cell and Wi-Fi consistency checks**, which are absent from `ARCHITECTURE.md` §5 and §3.2 (`checks`).
  - `SPEC.md §11.1` line 292 budgets **210 KB/post** (150 KB + 60 KB), which matches §7.5, while the 2.3 KB/capture index figure remains unvalidated until `canbench` runs on 1M records.
- **No-comments lint (§14 line 646):** Enforcing zero code comments across cryptographic parsers (`c2pa_lite`, Key Attestation X.509 parsing, MMR proofs, fixed-point determinism) risks introducing subtle maintenance bugs when invariants aren't self-evident from types alone. Consider scoping any comment lint to disallow *redundant* comments while permitting invariant/"why" comments.

---

## 7. Prioritized Action Items Before M0/M1

| Priority | Action | Sections Affected |
|---|---|---|
| **P0** | Fix `judge()` verdict logic: distinguish `Original` vs. `Derivative` in both C2PA and `exact` paths; verify enrolled unanchored captures as `SEALED_UNANCHORED` (and reconcile with the 4×4 matrix); specify `key_revoked_after_anchor` handling. | §3.2, §6.5, §12 |
| **P0** | Unify C2PA/JUMBF parsing (`c2pa_lite` + JUMBF `APP11` exclusion check) inside `witness-core` (or define the exact cross-platform seam) so `aarch64` and `wasm32` stay 100% deterministic and `@contentauth/c2pa-web` isn't redundant/divergent. | §2, §3.2, §6.5, §8.2 |
| **P0** | Fix Key Attestation quotas (`RKP intermediate` vs. device batch cert, replace global `FACTORY` bucket), keep leaf validity checks on `FACTORY`, add pre-`.await` reservation in `registry.enroll`, and define server-enforced `Retired` key transitions. | §4.1, §4.2, §7.2 |
| **P0** | Replace `KeyInfo`'s impossible "keyed MMR" with a certified `RbTree`/SMT keyed by `key_id` + append-only history if needed. | §7.2 |
| **P1** | Fix `pages` scaling: remove per-`case_id` certified entries (use one `/v/*` wildcard), resolve `post_upgrade` `HttpCertificationTree` rebuild vs. §7.1, and plan `pages` capacity/sharding. | §7.1, §7.4, §7.7 |
| **P1** | Fix `index` `u32` overflow key bit-packing, deduplicate `(local_ix, shape)` before `StableVec` reads, guard the 5s `registry` drain timer against re-entrancy, and cap `Watermark` list flooding. | §7.2, §7.3 |
| **P1** | Encode `media-N` shard routing into `case_id` (or `registry.shards()`), and clarify off-chain git root publication. | §7.2, §7.5, §7.7, §9 |
| **P1** | Fix `case.witness.app` CSP (`'wasm-unsafe-eval'`, SRI/importmap for dynamic modules, `img-src blob:`), stop idle 62 MB prefetch on `/p/` views, and guard against Rust `fmadd` float divergence across `aarch64`/`wasm32`. | §6, §8.1, §8.3 |
| **P2** | Add user-authenticated deletion for erasable `pages`/`media`/`index` data, design Witness Plus RevenueCat-to-ICP entitlement & 25 GB storage, cap CameraX capture resolution against 50 MP OOM, resolve `@Generable` vs. data-driven templates, and sync `SPEC.md` (`§4.3`, `§11.1`). | §5, §10.1, §10.3, `SPEC.md` |
