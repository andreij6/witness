# Spec Review & Critique: Witness (`docs/SPEC.md`)

**Reviewed document:** [SPEC.md](file:///Users/andrejones/Desktop/workspace/projects/witness/docs/SPEC.md) (367 lines, status: concept / pre-build)  
**Scope:** Product clarity, cryptographic & verification architecture, security/anti-spoofing, ICP canister feasibility, tech stack risk, compliance, and scope consistency.

---

## Executive Summary

`docs/SPEC.md` is an unusually clear, grounded, and honest pre-build product spec. Its strongest qualities are **disciplined problem framing** (explicitly separating what Witness proves from what it cannot prove), **pragmatic platform splitting** (native Android for hardware-attested capture vs. zero-install web WASM for verification), and **early attention to Google Play policy, privacy, and unit storage economics**.

However, before engineering starts, the spec has **several load-bearing contradictions and underspecified mechanisms** that will block implementation or break core promises if left unresolved:

1. **Verdict boundary ambiguity (`Matches an original, with changes` vs. `Altered`):** The spec does not define how verification distinguishes benign social-media transformations from malicious localized tampering, nor where the reference image comes from when originals are supposedly never uploaded by default.
2. **Offline capture vs. online attestation (`Play Integrity` + clock skew):** Binding a live Play Integrity check to *every* capture contradicts the offline/airplane-mode capture guarantee.
3. **Screenshot matching vs. PDQ limitations:** Promising 100% match rates on cropped social-media screenshots using PDQ + TrustMark ignores that global PDQ hashes degrade sharply under arbitrary cropping and UI chrome unless a crop/sub-image localization pipeline is specified.
4. **Private case link encryption conflict (`#fragment` vs. `vetKeys` vs. `Later` scope):** Sections 6.3, 7, 9, 10, and 12.2 contradict each other on whether encrypted case links are MVP or "Later", and whether decryption keys live in URL `#fragment`s or ICP `vetKeys`.
5. **Uncapped "never expire" storage liability on Witness Plus:** Promising non-expiring full-resolution photo/video proof pages for $2.99/mo ($24.99/yr) creates unbounded ICP storage debt if users upload heavy media or churn.
6. **Bleeding-edge client stack & browser download weight:** Shipping a ~5 MB Compose Wasm bundle plus a ~47 MB TrustMark ONNX model (plus `onnxruntime-web`) directly threatens the "<10 seconds" mobile web check goal.

---

## 1. What Works Well (Strengths)

- **Intellectual honesty about trust boundaries (§5):** The *Proves / Does not prove* table and plain acknowledgment of the analog hole (photographing a screen) and watermark stripping build real credibility and protect against overclaiming (§12.1).
- **Defense-in-depth against metadata stripping (§4.4):** Combining C2PA + invisible watermark (TrustMark) + perceptual hash (PDQ) + ICP timestamp anchoring directly solves the real-world failure mode of EXIF/C2PA on social platforms.
- **Privacy-preserving default on-chain footprint (§4.9, §11.1):** Anchoring 1–2 KB proof records rather than raw photos keeps ICP storage tiny (~$10/yr per 1M captures) and avoids turning a verification ledger into a public photo dump.
- **Pragmatic AI degradation (§4.6):** Treating devices *without* Gemini Nano as the normal baseline—and constraining on-device LLM output to typed fields mapped onto a fixed Compose component catalog—prevents AI availability or hallucination from breaking core workflows.
- **Early compliance hygiene (§12):** Accounting for Google Play Billing boundaries (no crypto/cycles in-app, off-app B2B purchases), Photo Picker instead of broad storage permissions, WorkManager instead of foreground services, and certified HTML first-paint for link previews and accessibility.

---

## 2. Critical Architectural & Security Gaps

### 2.1 How does Witness distinguish `Matches an original, with changes` from `Altered`? (§4.8, §13)
- **The gap:** Section 4.8 defines:
  - `Matches an original, with changes`: *"Cropped, filtered, recompressed or edited. Witness shows the original side by side and lists the differences. This is the normal outcome for social posts."*
  - `Altered`: *"A sealed file whose content was changed after capture"*
  - Section 13 adds: *"A tampered sealed file is **always** reported as Altered, and an untouched one is **always** Original."*
- **Why this breaks down:**
  1. **Attacker downgrade vector:** If `Altered` only triggers when a C2PA/Witness file seal is present and broken (e.g. modified JPEG bytes), an attacker can edit a photo in Photoshop, strip C2PA/EXIF (or take a screenshot of their edit), and submit it. Without a broken C2PA wrapper, it still matches the original via PDQ/watermark—turning a malicious forgery (`Altered`, red) into `Matches an original, with changes` (reassuring blue)!
  2. **Missing reference pixels for diffing:** Section 4.9 says *"Photos stay on your phone unless you choose to host them"*, and Section 11.1 says full-res media is *"paid tiers only"* (with only a ~60 KB WebP preview for public posts). Yet the Check flow (§4.8, Demo Step 2 & Step 6, `04-check.png`, `11-altered.png`) shows the **sealed original side-by-side**, measures *"About 25% of the frame removed"*, and even localizes *"pixels changed in 1 region"* (a 1-digit license plate swap `7MNV562` → `7MNV862`). You cannot compute pixel-level localized diffs or display high-res comparisons in a third-party browser if the original image never left the photographer's phone or only exists as a 60 KB thumbnail!
  3. **Privacy-blur contradiction (§4.5):** If a photographer blurs a bystander's face or neighbor's house number in Post Composer (§4.5) and posts it, showing the *unblurred* original publicly on `/check` defeats the privacy blur. Conversely, if only the blurred copy is public, `/check` cannot honestly label it *"Sealed original"* while listing *"2 blurs declared"* under *"What changed"*.
- **Recommendation:**
  - Define an explicit **verdict state machine**: e.g., broken cryptographic seal OR localized structural/semantic divergence (beyond global crop/scale/recompression) → `Altered`; intact watermark/global perceptual match with benign global transforms (crop/scale/JPEG re-encode) → `Matches, with changes`.
  - Specify whether **block-level/patch hashes** (e.g., an 8×8 grid of micro-hashes stored inside the ~2 KB proof record) are used to localize changed regions without storing full-res originals on-chain.
  - Specify **redaction privacy semantics**: declared privacy blurs should produce a *redacted public reference* so unredacted faces/plates/house numbers are never exposed to strangers on `/check`.

### 2.2 Offline Capture vs. Play Integrity & Key Attestation (§4.2, §4.7, §9)
- **The gap:** Section 4.2 states: *"The key is created in StrongBox or the TEE with Google-signed Key Attestation, plus a Play Integrity check bound to each capture."* Meanwhile, §4.7 and §9 Demo Step 5 require full **airplane-mode / offline capture**.
- **Why this breaks down:**
  - Google's **Play Integrity API requires a live network call** to Google Play servers to fetch an integrity token. You cannot bind a fresh Play Integrity verdict to a capture taken in airplane mode.
  - Even online, Standard/Classic Play Integrity tokens are encrypted/signed JWE/JWS structures meant to be decoded via Google's server-side API (or a Play Console-managed decryption key). **An ICP canister cannot safely hold an OAuth service account credential or static decryption secret** without threshold cryptography or an off-chain proxy, neither of which is specified in §15.
  - **Local clock manipulation:** In offline mode, an attacker can roll back Android's system clock before shooting. The hardware-signed capture timestamp will read `T_claimed`, while the ICP anchor timestamp later reads `T_anchor`. The spec has no rule for flagging large `T_anchor - T_claimed` deltas (e.g., >24h or mismatched against GNSS GPS time).
- **Recommendation:**
  - Separate **enrollment-time / periodic Play Integrity** (bound to the device's hardware-attested key certificate) from **offline per-capture hardware signing** (which can also sign raw GNSS time alongside wall-clock time).
  - Explicitly document how the **Registry Canister** verifies Android Key Attestation X.509 chains (pinned Google root CAs + periodic HTTPS outcall to sync Google's CRL at `android.googleapis.com/attestation/status`) and how Play Integrity is verified without leaking secrets inside canister source/state.

### 2.3 Screenshot Matching Reliability vs. PDQ & TrustMark Limits (§4.4, §13)
- **The gap:** Section 13 requires: *"A screenshot of an Instagram post of a Witness capture is matched to its original **every time** in testing."*
- **Why this is risky:**
  - **PDQ is not crop-invariant.** Meta's PDQ computes a global 256-bit DCT hash over the entire image. A phone screenshot containing Instagram UI chrome (`@stormwatch.den`, status bar, like bar, caption) plus a 4:5 or 1:1 crop of a 3:4 photo will produce a **completely different PDQ hash** (Hamming distance far above threshold).
  - **Watermark resilience:** TrustMark degrades under severe cropping, aspect-ratio changes, or screen-DPI scaling unless synchronization/tiling or pre-crop detection is handled explicitly.
- **Recommendation:**
  - Add an explicit **client-side preprocessing stage** in `witness-web` / Android Share Check: detect and extract the largest photographic bounding box (removing social UI chrome/letterboxing) before running PDQ + TrustMark, and consider pairing PDQ with a crop-resilient local feature descriptor or multi-crop indexing if screenshots must match *"every time"*.
  - Relax the binary *"every time"* success criterion (§13) into measurable target metrics (e.g., ≥98% recall at <0.1% false-positive rate across an explicit test matrix of Instagram, X, TikTok, and WhatsApp re-encodes and crops).

### 2.4 Contradictions in Private Case Links (`#fragment` vs. `vetKeys` vs. MVP Scope) (§6.3, §7, §9, §10, §12.2)
- **The gap:**
  - §6.3 & §12.2 say private case links `/v/<id>` put the access key in the URL `#fragment` so the server never sees it.
  - §7 says private case links use ICP's **vetKeys**, and access can expire.
  - §10 puts **"Encrypted case links with expiry (vetKeys)"** in **Later** (post-MVP), **yet** §9 Demo Step 6 includes *"Share a case link and open it on a laptop to see the green verdict"* inside the 2-minute MVP demo!
  - Screen `09-share-case.png` also adds a **"Recipient must sign in"** toggle alongside expiry (`24 h / 7 days / 30 days / Never`).
- **Recommendation:**
  - Clarify the exact crypto split:
    - **MVP:** Symmetric AES-GCM encryption with the key in `#fragment` (zero server knowledge) + canister-enforced TTL on serving the encrypted payload (noting that once fetched before expiry, a recipient already possesses the decrypted copy).
    - **Later (`vetKeys`):** Identity-bound access (`Recipient must sign in` via Internet Identity) where `vetKeys` derives keys only after authentication and before expiry.
  - Move the chosen MVP case-link mechanism explicitly into §10 (MVP) so §9 and §10 do not contradict each other.

### 2.5 Video & Audio Sealing Scope & Technical Gaps (§4.2, §10, §11.2)
- **The gap:** §4.2 introduces *"Photo, video and audio capture"*; §10 moves *"Video and audio sealing"* to **Later**; §11.2 lists *"video and audio sealing"* as a paid **Witness Plus** feature. Meanwhile, §15 (Building blocks) specifies zero audio/video watermarking or fingerprinting libraries (TrustMark and PDQ are image-only).
- **Recommendation:** Align §4.2, §10, and §11.2: state clearly that MVP is **Photo-only** (and remove or mark `VIDEO` / `AUDIO` as "Coming soon" if not in MVP), and add a brief note on how video/audio sealing will work post-MVP (e.g., keyframe PDQ/C2PA chunking, audio fingerprinting).

---

## 3. Infrastructure, Scale & Unit Economics Gaps

### 3.1 "Never Expire" Storage Liability on Witness Plus (§11.1, §11.2)
- **The gap:** Witness Plus ($2.99/mo or $24.99/yr) promises *"full-resolution proof pages that never expire"* plus *"video and audio sealing"* and *"backup and multi-device sync"*.
- **Math problem:** ICP storage costs ~$5/GB/year (§11.1). A single creator or field adjuster capturing 1,000 high-res photos/yr (3–5 MB each) + videos quickly exceeds 10–50 GB/year ($50–$250/yr in ongoing storage alone). Worse, if *"never expire"* applies even after a user cancels their $2.99 subscription, Witness absorbs perpetual, unbounded ICP cycle burn for churned users.
- **Recommendation:**
  - Cap included storage per Witness Plus tier (e.g., 10 GB or 25 GB included; additional storage packs or Teams tier above that).
  - Clarify that **proof records** (1–2 KB hashes/timestamps) never expire, while **hosted full-resolution media** stays hosted while the subscription is active (with a grace window / local export on cancellation).

### 3.2 Free-Tier Spam, Abuse & Moderation (§4.9, §11.1, §12.2)
- **The gap:** Free tier allows *"Unlimited sealed captures, anchoring, checking, and proof pages with previews"* with *"No account required"*. Each proof-page preview is ~60 KB (~$300/yr per 1M posts).
- **Risk:** Even with Play Integrity / Key Attestation, an automated farm or malicious user can flood the canister with 60 KB WebPs (storing illegal imagery, CSAM, copyrighted material, or spam) and generate public `witness.icp0.io/p/...` URLs. Furthermore, §12.2 promises a *"report link, a takedown process"*, which requires mutable deletion of previews/pages on ICP.
- **Recommendation:**
  - Add per-device rate limits / proof-of-work or attestation-key quotas on free preview uploads.
  - Explicitly state in §4.9 / §11.1 that **hosted WebP previews and case blobs are stored in erasable canister state** (to honor takedowns, CSAM compliance, and GDPR/account deletion), whereas only non-reversible cryptographic hashes/Merkle roots are permanent.

### 3.3 ICP Canister Architecture & Scale (§14, §15)
- **The gap:**
  1. **Single "Registry canister" bottleneck:** Putting Merkle anchoring, PDQ/watermark indexes, Android attestation verification, certified HTML generation, AND 60 KB WebP previews (~60 GB per 1M posts) into one "Registry canister" will hit single-canister heap/stable-memory limits and update/query throughput ceilings.
  2. **PDQ nearest-neighbor search in WASM:** Linear Hamming scan over 1M+ 256-bit PDQ hashes will exceed ICP's per-query instruction limit (~5B instructions). Multi-index hashing (MIH) or sharded index buckets in stable memory should be called out as an explicit MVP architectural requirement, not just a vague risk.
  3. **Custom domain strategy:** Hardcoding `witness.icp0.io/p/...` in watermarks/QR codes/C2PA manifests ties permanent physical/social links to raw ICP canister URLs, which look unfamiliar/suspicious to non-crypto users. Plan for a custom domain (e.g. `wtn.sh` / `witness.org` via ICP custom domain routing) before issuing permanent links.

---

## 4. Client Tech Stack & UX Feasibility Risks

### 4.1 Mobile Web Payload Size vs. `< 10 seconds` Goal (§13, §14, §15)
- **The gap:** Checking a screenshot on the web requires downloading:
  - Compose Multiplatform Wasm bundle: ~5 MB compressed (§14)
  - `onnxruntime-web` WASM runtime: typically ~10–20 MB
  - TrustMark ONNX decoder weights: ~47 MB (§14)
- **Total first-visit download on `/check`: ~62–72 MB.** On a mobile browser over cellular data (e.g., an iPhone user tapping a check link from X), downloading and initializing 60+ MB of WASM + ONNX will easily exceed the **10-second** success criterion (§13) and spike ICP canister egress cycles!
- **Recommendation:**
  - Make **PDQ + C2PA** (`witness-web` + `c2pa-web`, only a few MB) run immediately as Stage 1, and lazy-load the 47 MB TrustMark model *only* when PDQ/C2PA finds no match (or progressively in the background), or evaluate a smaller quantized/distilled watermark decoder for web.
  - Verify whether `onnxruntime-web` and TrustMark weights are self-hosted on the Web Canister (required by §12.2's *"No third-party requests"* rule!).

### 4.2 A2UI (`1.0.0-alpha02`) & Bleeding-Edge Stack Risk (§14, §15)
- **The gap:** The stack stacks several pre-release / alpha / beta dependencies simultaneously: Kotlin 2.4, Compose Multiplatform 1.12, Navigation 3, Room 3, `c2pa-web` (pre-1.0), and **A2UI Compose (`1.0.0-alpha02`)**.
- Specifically on **A2UI**: §4.6 explains that Gemini Nano *only outputs a small typed answer* (template ID, caption, alt text, detected fields) and Witness renders from a **fixed component catalog**. Introducing an `alpha02` dynamic UI framework just to map typed JSON onto a fixed set of 3 templates (car accident, property damage, rental condition) adds high API churn and debugging cost with minimal benefit over a simple `@Serializable` sealed class rendered directly by Compose.

### 4.3 Minor Spec Omissions & Internal Inconsistencies
- **Missing screens / flows in §8 & §10:**
  - **Witness Plus Play Billing purchase flow** is in MVP (§10), yet has no screen in §8 and no UI entry point in the design.
  - **"Move a post into a case at any time"** (§4.1) and **"Imports from the gallery marked unverified"** (§4.2) have no specified UX flow.
  - **Safety mode** (§4.9: app lock, quick-hide gesture, strip precise location) is described as a feature but absent from Scope (§10), Screens (§8), and Compliance (§12.1 — note that disguising/hiding app icons or emergency-hide gestures must be handled carefully with Play policy).
  - **Indoor GNSS fallback (§4.3):** Raw GNSS fails indoors (e.g., *"Kitchen ceiling leak"*). The spec needs to clarify how Wi-Fi/cell cross-checking works *without* leaking user BSSIDs/cell IDs to third-party geolocation APIs (preserving §4.9 privacy), or whether indoor captures automatically drop to `Low` / `Medium` confidence.
  - **RevenueCat Data Safety (§11.2, §12.1):** Using RevenueCat means device identifiers and purchase telemetry go to a third party; §12.1's Data safety bullet only mentions hashes/fingerprints/watermark IDs and needs to include RevenueCat + Play Integrity telemetry.
  - **Brand name conflict (§14):** Rightly flagged—`"Witness"` collides directly with [WITNESS.org](https://witness.org), the prominent human-rights video-verification nonprofit (which literally works in the exact same space of citizen-journalism media authentication!). Rename early rather than treating it as a minor post-MVP question, because renaming after embedding URLs/watermarks/C2PA namespaces is costly.

---

## 5. Prioritized Action Items for `SPEC.md`

| Priority | Action | Sections Affected |
|---|---|---|
| **P0** | Define the exact algorithmic & cryptographic boundary between `Matches, with changes` and `Altered`, and specify where comparison reference images/patch hashes live (including privacy rules for declared redactions). | §4.4, §4.5, §4.8, §11.1, §13 |
| **P0** | Reconcile offline capture with Play Integrity & Key Attestation (move Play Integrity to enrollment/sync; specify canister-side CRL sync & clock-skew policy). | §4.2, §4.7, §15 |
| **P0** | Add a screenshot photo-region detection/cropping step before PDQ/TrustMark, or revise the *"every time"* screenshot matching claim to realistic benchmarks. | §4.4, §13, §14 |
| **P1** | Resolve the MVP vs. Later contradiction on Private Case Links (`#fragment` encryption vs. `vetKeys`) and Video/Audio sealing. | §4.2, §6.3, §7, §9, §10, §11.2 |
| **P1** | Split TrustMark model loading (lazy fallback after fast PDQ/C2PA) to protect the <10s web check target, and re-evaluate whether A2UI `alpha02` is justified for 3 fixed templates. | §4.6, §13, §14, §15 |
| **P1** | Cap Witness Plus storage & clarify post-cancellation retention ("proofs never expire, hosted media requires active tier"); add free-tier anti-spam/takedown mechanics. | §4.9, §11.1, §11.2, §12.2 |
| **P2** | Resolve the `"Witness"` brand collision with WITNESS.org before creating permanent C2PA/URL namespaces, and add Safety Mode / Gallery Import / Plus Billing to §8 & §10. | §4.1, §4.2, §4.9, §8, §10, §14 |
