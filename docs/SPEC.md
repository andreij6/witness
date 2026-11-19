# Witness — Product Spec

> **Proof, not just pictures.** Witness is an Android camera app, with a companion website, whose photos anyone can verify: captured on a genuine phone, at the time and place claimed, and untouched since. The proof survives being posted to social media, compressed, cropped or screenshotted.

*Working title. Status: MVP built and running end to end on an Android emulator against a local Internet Computer replica (development keys, DEV badge). Not yet deployed to mainnet. What works today and what's pending: [docs/dev/STATUS.md](dev/STATUS.md).*

---

## 1. The problem

Photos and video used to settle arguments. Generative AI ended that. Any image can now be faked in seconds, so real moments get dismissed as "probably AI" and fakes get believed.

- **On social media**, a real photo of a storm, a protest or a celebrity sighting gets called fake in the replies. A fake one goes viral before anyone can check.
- **In disputes**, a driver's accident photos, a tenant's move-out photos or an adjuster's report get challenged as doctored.
- **Reposts erase context.** Nobody knows who took the original, when, or whether it was edited.

Metadata doesn't help. EXIF is trivial to edit, and every social platform strips it on upload anyway.

## 2. What Witness does

Witness turns any capture into something **provably real**, then keeps that proof attached wherever the image travels.

The core loop:

```
Capture  →  Seal  →  Prepare  →  Anchor  →  Share  →  Verify
```

| Step | What happens |
|---|---|
| **Capture** | You take photos in the Witness camera. Video and audio come later. |
| **Seal** | At the shutter, the phone's security chip signs the media together with time, location and device facts. An invisible watermark and a visual fingerprint are added so the proof survives social media. |
| **Prepare** | The on-device AI gets the capture ready for where it's going. For a post, it offers blurs, alt text and a caption. For a case, it builds the right report and coaches you on missing shots. |
| **Anchor** | The proof is timestamped on the Internet Computer. Only the proof is anchored, never the photo itself. This proves it existed at that moment. It works offline and anchors when you're next online. |
| **Share** | Post anywhere, such as Instagram, X, TikTok or WhatsApp, or send a private case package or link. |
| **Verify** | Anyone can check a post, a screenshot or a file and see the sealed reference next to it, with the differences shown. |

## 3. Who it's for

| Persona | Typical use |
|---|---|
| **Everyday poster** | Weather, events, sightings, "you won't believe this" moments they want believed |
| **Creator / citizen journalist** | Building a reputation for real footage; proving they shot it first |
| **Viewer / fact-checker** | Checking whether a viral image is real before sharing it |
| **Person in a dispute** | Car accidents, rental move-in and move-out, deliveries, contractor work |
| **Field professional** | Insurance adjusters, inspectors, utility crews, claims photographers |
| **Organization** | Newsrooms, trust & safety teams and insurers verifying content at scale |

## 4. Key features

### 4.1 Two modes, one camera
- **Post** (the default) is a fast capture meant for sharing. It goes from sealed photo to the share sheet in a few taps.
- **Case** is for evidence. Captures go into the **active case**, which is shown on the camera and can be switched or created from there. The AI builds a report, and you get exportable packages.
- You can move a post into a case at any time from the item's proof screen. The proof is the same either way.

### 4.2 Verified camera
- **Photo capture inside the app** for the MVP. Video and audio come later (see §10).
- **Gallery imports** are allowed but clearly marked **Unverified**. They can be checked, but never sealed as captures.
- Every capture is sealed immediately with a key that lives in the phone's secure hardware (StrongBox, or the TEE) and can't be extracted, not even by Witness.
- **Device attestation works offline.** Each device key carries Google-signed **Key Attestation**, which the registry canister verifies on-chain against Google's root certificates and revocation list. Witness then issues the key its own Content Credentials signing certificate. It proves:
  - the key is hardware-bound
  - the phone's bootloader is locked and its OS verified
  - the app is the genuine Witness build (package name and signing certificate)
- **Development devices are labeled.** Local builds can enroll the Android emulator, whose keys aren't hardware-attested by Google. Proofs from those keys carry a **DEV** badge and say "development device", never "genuine phone", on every screen and page. Production builds never accept them.
- **Play Integrity is not part of the proof.** Its verdicts need a network call and server-side decryption that a canister can't do safely. It may be added later through a separate verifier, labeled as such.
- **Time is honest:**
  - Each capture signs the phone's clock, the satellite (GNSS) time when a fix is available, and a monotonic counter.
  - The proof always shows the **anchor time** as a guaranteed bound: "existed no later than…".
  - If the claimed capture time and the anchor are more than 24 hours apart, or the phone clock disagrees with GNSS time, the proof says so.
- Every proof lists **which checks ran** on that phone and how strong each was. Phones differ, so proofs never claim more than the phone could actually show.

### 4.3 Location you can trust
- Ordinary GPS is easy to spoof.
  - Outdoors, Witness reads the **raw satellite signals** the phone receives and checks them for signs of spoofing, along with Android's mock-location flag.
  - **On the device only,** the cell network's country must match the satellite fix. Witness never sends nearby network IDs to any location service.
- Each capture shows a plain-language **location confidence**, with the reason:
  - *High*: raw satellite checks passed.
  - *Medium*: a fused fix with no satellite check, such as indoors.
  - *Low*: weak or inconsistent signals.
- For public posts, exact location is hidden by default and shown only as a city or area. The precise location stays provable in the sealed original.
- **Precise location is never stored on-chain.** Proof records hold only a salted commitment, plus an optional coarse area. The owner can prove the exact spot by sharing the sealed file.

### 4.4 Proof that survives social media
Platforms strip metadata, recompress everything, crop to their own shapes and wrap screenshots in their UI. Witness doesn't rely on the file staying intact:

- **Invisible watermark (primary).** An imperceptible mark carries a short ID that survives compression, resizing and moderate cropping. It uses Adobe's open TrustMark models, which are fetched separately and aren't in the build yet; until they are, the app and site say "watermark unavailable" and rely on exact hashes and fingerprints, so heavily cropped copies may come back *No Witness record*.
- **Visual fingerprints (fallback).** At capture, Witness stores perceptual fingerprints (PDQ) of the full frame *and* of about 15–19 common social shapes (1:1, 4:5, 3:4, 9:16, 16:9, 1.91:1) at several positions, because real crops aren't always centered. A single fingerprint isn't crop-proof, so these cover the common cases. Arbitrary crops rely on the watermark.
- **Screenshot clean-up first.** Before matching, the checker finds the photo inside a screenshot and cuts away app chrome: status bars, usernames, like buttons, captions.
- **Content Credentials (C2PA)** are embedded too, written by Witness's own small Content Credentials module (the same code reads them back on the phone, the website and the registry). They are checked against the reference C2PA reader in tests, so platforms that read them can show their own credentials badge.
- **Public proof page.** Each post can get a short link (`witness.app/p/…`, placeholder domain) to drop in a caption or bio. It shows the verdict, the capture time, a coarse location and a preview.
- **First seen.** The anchor proves who captured it and when. When something goes viral, the original creator can show it was theirs first.

### 4.5 Honest edits and redactions
- Edits made in the app are **declared, not hidden**: crop, straighten, exposure, light filters, and blurring faces, plates or house numbers.
  - The edited version is sealed as a **derivative** of the original, signed by the same key, with the list of edits.
  - Viewers see exactly what changed.
- **Redactions stay private.** When a post has declared blurs, the public reference image (preview and proof page) is the **redacted** version. The unredacted original is never shown to strangers, but its sealed hash still proves it exists, and the owner can reveal it if they choose.
- Edits made in other apps don't break the link. The copy is traced back to its original, and the verdict rules below decide how it's labeled.

### 4.6 On-device AI assistant
A language model runs **entirely on the phone**. No cloud AI is used and no media is uploaded for this step.

- **The model is Gemini Nano,** which is built into the OS on supported phones, so it adds nothing to the app's size.
- **Report screens are A2UI documents, not code.**
  - Each situation (car accident, property damage, rental condition) is a declarative template of components from Witness's fixed catalog: checklists, fields, pickers and the impact diagram.
  - New situations can be added as data without an app update.
  - The model fills a small, typed answer: which template, field values, a caption, alt text. Witness binds that answer into the template.
  - A bad answer can only pick the wrong template or a wrong value, never produce a broken screen.
- **Most phones have no built-in model, so that's the normal case, not an edge case.** On those phones Witness picks templates with simple rules, and every feature still works.

- **In Post mode:**
  - Finds faces, license plates and house numbers and offers to blur them. Each blur can be undone individually.
  - Warns when a post would reveal a precise location.
  - Writes alt text and suggests a caption.
- **In Case mode:**
  - Recognizes the situation and builds the **right report**. A car accident brings up the other driver's plate, the insurer and an impact diagram.
  - Coaches you on missing evidence, such as *"Get a wide shot showing both cars."*
  - Drafts a neutral summary you can edit.
- Anything the AI generated sits on a visibly marked surface. Users can **edit or remove** it and **flag** bad output.

### 4.7 Home: your captures
- A private feed of everything you've captured, with optional **cases** as folders and a **New case** action.
- Each item shows its seal, its location confidence and its anchoring status: *Sealed*, *Queued* or *Anchored*. Gallery imports show *Unverified*.
- Works fully offline. Anchoring catches up automatically.

### 4.8 Share and verify
- **Share a post** through the system share sheet. The watermark and fingerprints go along automatically.
- **Share a case** as an export package (ZIP with the sealed files and proof records, plus a PDF report on Plus) or a **private case link**:
  - The case is encrypted on the phone (AES-GCM) and uploaded to a media canister. The key (and a hash of the case, used to detect tampering) goes in the link's `#fragment`, which browsers never send to a server, so Witness can't read shared cases.
  - A case link holds up to 16 MB of encrypted photos free, or up to 256 MB on Witness Plus (counted against the Plus storage allowance).
  - Links expire: the canister deletes the encrypted payload 30 days after sharing, and the link page shows that date. Anyone who opened it before then may have kept a copy.
- **Check anything:**
  - In the app, use the **Check** tab (pick a photo or paste a link) or **Share → Witness** from any other app.
  - On the **Witness website**, from any phone (including iPhones) or computer.
- Verification works in any modern browser without the app. Only **capturing** needs the Android app.

#### Verdict rules
A check first looks for cryptographic proof, then for a match: the exact file or pixel hash, then the watermark ID, then the fingerprints.
- **A match is only accepted once it's confirmed by aligning the copy with the reference.** A fingerprint that merely looks similar is never enough.
- The comparison uses the public reference image (the possibly redacted preview, or the case item) when one exists. Otherwise it uses a coarse **region grid** of about 200 tiny signatures stored with each proof.

| Verdict | Rule |
|---|---|
| ✅ **Original** | **Cryptographic only:** the file's Content Credentials are signed by an enrolled Witness device key and its data hash matches, or the exact file or pixel hash is registered. It also covers versions only resized or recompressed for the web; edited versions (crop, blur, filters) are never *Original*. |
| ✏️ **Matches an original, with changes** | Confirmed match, and the only differences are *global*: crop, scale, recompression, color and filters, declared edits. Changed areas are listed, and the reference is shown side by side. This is the best a copy can get, even when no visible change is found. |
| ⚠️ **Altered** | The seal is broken, or a confirmed match has one or more *local* changed regions (small, bounded areas surrounded by matching content). The changed regions are highlighted. |
| ❔ **No Witness record** | No confirmed match. Not proven, which is not the same as proven fake. |

- **Sealed but not yet anchored:** a file that is validly sealed by an enrolled phone but hasn't reached the registry (e.g. shared straight from an offline phone) shows *"Sealed · not yet anchored"*.
- **When regions can't be compared** (heavy crop, low resolution, overlays), the verdict stays *Matches an original, with changes*, and Witness says plainly that **some regions couldn't be compared**.
- **When no public reference image exists** (a capture that was never posted), the verdict is capped at *"Matches a sealed photo, with no public reference"*. Capture facts are shown instead of an image.
- **A watermark on unrelated content** gives *No Witness record*, with a warning that it carries a watermark from a different photo. No reference is shown, so a real person's proof can't be attached to someone else's image.
- **Fine edits need the reference.** A changed license-plate digit is caught when a reference image exists. The coarse grid alone only catches larger edits, about half a grid cell or more. On very small copies (under about 256 px wide) the grid only reports changes larger than one JPEG compression block, so small edits there show as *Matches an original, with changes*, never *Original*.

### 4.9 Privacy and safety
- **No account required.** An optional Internet Identity sign-in enables sync and a creator profile.
- **Proof says "a genuine phone captured this", not who you are.** Linking your identity is opt-in.
- **Permanent versus erasable:**
  - **Permanent:** only minimal proof records (hashes, commitments, watermark ID, key ID, timestamps) and their certified anchors.
  - **Erasable:** fingerprints and region grids (they can be partly reconstructive, so they're treated as personal data), previews, proof pages, encrypted case payloads and hosted media. These can be deleted for takedowns, legal requests or account deletion.
- **Abuse limits.** Free previews and proof pages are rate-limited per attested device key, and only Witness-sealed captures can become proof pages. Arbitrary uploads are never accepted.
- **Safety mode** (after MVP) for at-risk users: an app lock and stripping precise location from anything shared while keeping it provable.
- **Diagnostics are opt-in.** The Android app can send crash reports and usage counts (Firebase Crashlytics and Analytics), but only after the user turns them on in Settings, and the events never contain record IDs, locations, hashes or images. The website collects nothing.

## 5. What Witness proves, and what it doesn't

Being honest about limits is part of being trustworthy.

| Witness **proves** | Witness **does not prove** |
|---|---|
| The media came from a genuine phone running the real Witness app | That the scene wasn't staged |
| It existed no later than its anchor time; the capture time is shown with how it was checked | Who was holding the phone, unless they chose to link their identity |
| What changed between the original and a copy, region by region where the grid allows | That every relevant moment was captured |
| Its location, within the stated confidence | Legal admissibility, which varies by jurisdiction |

Known gaps, stated plainly in the app:
- **Stripping the watermark on purpose** and cropping hard can give *No Witness record*. It never gives a false *Original*.
- **Photographing a screen** produces a genuine capture of fake content. Witness flags likely screen captures (moiré, flat depth), but it can't guarantee catching them. The flat-surface check is strongest on phones with a depth sensor.

## 6. One codebase: an Android app and a website

Witness is built with **Compose Multiplatform** for two targets that share UI code: the **Android app**, where capturing happens, and the **website**, where anyone checks. Shared composables (verdict cards, chain of custody, proof details) look identical in both.

**iOS is out of scope for now.** iPhone users can check anything on the website, but capturing needs Android. iOS can't match Android's capture proofs (no raw satellite data, no general key attestation), and supporting it would add a community A2UI port and less mature Rust bindings to the critical path. The architecture leaves room to add it later.

### 6.1 Why capturing needs a native app

Each of the following is impossible from a browser today:

- **Hardware-backed signing and attestation:** Android Keystore with Key Attestation.
- **Raw satellite measurements** for spoof detection. Browsers only expose a final, easily faked latitude and longitude.
- **Direct camera pipeline access** through CameraX, so frames are sealed and watermarked before any other app touches them.
- **The OS's built-in AI model** (Gemini Nano) working on the media fully offline.
- **System share integration** (Share → Witness from any app) and **reliable background anchoring**.

### 6.2 Why checking works on the web

Checking needs no secrets and no hardware, only math, which the browser can do:
- It decodes the image locally.
- It computes the fingerprint with Witness's Rust code compiled to WebAssembly.
- It reads the watermark and Content Credentials.
- It asks the index canisters for a match and reads the certified record from the registry.

The image itself never leaves the visitor's device.

### 6.3 What the website is

| Page | What it does |
|---|---|
| **Proof page** `/p/<id>` | The public page behind a post's proof link: verdict, capture facts, coarse location, *first seen* |
| **Case link** `/v/<id>` | A private case shared with a recipient; the access key is in the link's `#fragment`, so it never reaches a server |
| **Check** `/check` | Drop a screenshot or file, or paste a link, and get one of the four verdicts with the original side by side |

How each page loads:
- **Link previews and first paint come from real HTML.** Social apps and search engines don't run WebAssembly. So the pages canister answers `/p/<id>` with a small, certified HTML page that carries the preview tags (title, description, preview image) and a plain-text verdict summary. That summary is instantly readable by people, screen readers, translators and Ctrl+F.
- **The Compose app then loads** for the interactive view.
- **Modern browsers** run the WebAssembly build. Older ones, including iPhones before iOS 18.2, get the automatic JavaScript fallback.

## 7. Why the Internet Computer

- **A neutral record nobody controls.** Not Witness, not the poster and not the other party. That neutrality is the point when content is disputed.
- **A public lookup registry.** Canisters index watermark IDs and fingerprints, so any copy anywhere can be traced back to its sealed original.
- **Certified responses.** Verification results carry a cryptographic certificate, so the verify page itself can't be quietly faked.
- **Encrypted sharing.** Private case links are encrypted on the phone, and the canister only ever stores ciphertext. Later, ICP's on-chain key management (vetKeys) enables links that only a signed-in recipient can open.
- **Users never touch crypto.** Canisters pay for their own compute and storage. People just use an app.

## 8. Main screens

These match the frames in `design/witness.figcraft`. The frames are grouped in rows by flow, and renders are in `design/renders/`.

**A · Start and home**

| # | Screen | Purpose |
|---|---|---|
| A1 | Splash | Brand on ink |
| A2 | Welcome | What Witness proves, in three promises; skip straight to checking |
| A3 | Permissions | Camera (required) and location while in use (optional); what Witness never asks for |
| A4 | Device setup | Hardware key → Google attestation → Witness certificate; works offline and finishes later |
| A5 | Unsupported device | Why this phone can't seal, and that checking still works |
| A6 | Home · empty | First-run empty state |
| A7 | Home | Posts, an unverified import, cases, anchoring status |
| A8 | Anchoring status | Anchored and queued counts, the queue, recent anchors, key rotation |
| A9 | Search | On-device search across items and cases |

**B · Capture and post**

| # | Screen | Purpose |
|---|---|---|
| B1 | Camera · Post | Seal and location pills, auto-blur detection, Post/Case switch |
| B2 | Camera · sealed offline | A seal that's queued for anchoring |
| B3 | Post composer | Blurs (removable one at a time), location precision, alt text, caption, proof link |
| B4 | Edit photo | Crop, straighten, tone, blur; every edit declared |
| B5 | Location precision | Hidden, area, neighborhood, exact |
| B6 | Post shared | Proof link, QR code, first stats |

**C · Cases**

| # | Screen | Purpose |
|---|---|---|
| C1 | Camera · Case | Active-case switcher, new case, evidence coach |
| C2 | Case switcher | Choose which case captures go into |
| C3 | New case | Name and situation; report suggested on-device |
| C4 | Cases | All cases, with status and shared links |
| C5 | Case | AI-built A2UI report: checklist, generated form, draft summary |
| C6 | Report template | Change the situation template |
| C7 | Flag AI output | Report or remove a generated part |
| C8 | Export | ZIP of sealed files (free), PDF report (Plus) |
| C9 | Share case | Selection, redactions, expiry, end-to-end-encrypted link |

**D · Item and check**

| # | Screen | Purpose |
|---|---|---|
| D1 | Item proof | Checks with their strength, and the chain of custody |
| D2 | Item actions | Move, post, prove location, technical record, delete, erase public copies |
| D3 | Technical record | IDs, hashes, key, certificate, MMR inclusion |
| D4 | Prove location | Explicit, user-created proof of the exact spot |
| D5 | Check | Choose a photo, paste a link, recent checks |
| D6 | Checking | Staged progress |
| D7 | Check result | *Matches an original, with changes*, beside the sealed reference |
| D8 | Check · not yet anchored | Valid seal, anchor pending |

**E · Settings and account**

| # | Screen | Purpose |
|---|---|---|
| E1 | Settings | Account, Plus, privacy, deletion |
| E2 | Witness Plus | Plans, storage cap and retention, Google Play purchase |
| E3 | Internet Identity | Optional link for deletion anywhere, sync and Plus |
| E4 | Delete account | What's deleted and what stays |
| E5 | Privacy & data | Permanent, erasable and phone-only data; export; erase public copies |

**W · Website, mobile**

| # | Screen | Purpose |
|---|---|---|
| W1 | Proof page | Public post page: *Matches an original, with 2 declared blurs* |
| W2 | Web check | Choose or paste; checked in the browser |
| W3 | Web checking | Staged progress; the watermark model loads only if needed |
| W4 | Web · with changes | Web result with the reference beside it |
| W5 | Altered | A re-saved forgery, with the changed region highlighted |
| W6 | No record | What was checked, and what to do |
| W7 | Case link | 4 of 4 verified, the sender's AI summary, a verdict for each item |
| W8 | Case link expired | Expiry and deletion explained |
| W9 | Removed page | Tombstone after a takedown |
| W10 | Report | Report a page |

**X · Website, desktop**

| # | Screen | Purpose |
|---|---|---|
| X1 | Desktop · proof page | Proof page at 1280 px |
| X2 | Desktop · check | Drag-and-drop check with the result |
| X3 | Desktop · case link | Case header, verdict, item grid |

## 9. Demo scenario (2 minutes)

**Social**
1. Capture a hailstorm in Post mode. The AI offers to blur a neighbor's house number, so accept it, then post to Instagram.
2. Screenshot the Instagram post, crop it, and **Share → Witness**. The result is ✏️ *Matches an original, with changes*: cropped, recompressed, one declared blur. The sealed reference, with its blur, appears next to it.
3. Run a random image from the web through the Check tab. The result is ❔ *No Witness record*.

**Case**
4. Capture a mock fender bender in Case mode. The AI builds an accident report and asks for the other plate.
5. Capture in airplane mode: items show **Queued**, then flip to **Anchored** when the network returns.
6. Share an encrypted case link and open it on a laptop to see the ✅ *Original* verdicts. Then change the plate in one photo in an editor, screenshot it and check again: ⚠️ *Altered*, with the changed region highlighted.

## 10. Scope

**MVP (portfolio demo)**, an Android app and a website from one Compose Multiplatform codebase
- **Photo only.** Capture with a hardware seal, on-chain-verifiable Key Attestation, watermark, multi-crop fingerprints and region grid
- Location confidence from raw satellite data, on supported phones
- Post mode with AI blur suggestions and alt text, shared through the system share sheet
- Check through the in-app Check tab, Share → Witness and the website, covering all four verdicts and the verdict rules in §4.8
- Public proof pages with previews, served on a custom domain
- Case mode with an active-case switcher and the AI report for 3 situations: car accident, property damage, rental condition
- Encrypted private case links (key in the `#fragment`, expiry enforced by the canister)
- Offline feed with background anchoring to ICP canisters
- Settings with the Witness Plus purchase flow (Play Billing via RevenueCat)

**Built so far** (local replica, emulator): capture with a hardware-key seal and Content Credentials, enrollment with a DEV-labeled emulator key, fingerprints and region grid, background anchoring, Post mode with real blurs and a shared preview, the public proof page, the Check tab, Share → Witness and `/check`, Case mode with the rules-based report (Gemini Nano not yet tried on a device), encrypted case links, and per-item erase. **Still pending:** the watermark models, a physical-phone run with real Google attestation, custom domains and mainnet deployment, Plus billing (RevenueCat keys), Internet Identity, and the full benchmark. Details in [docs/dev/STATUS.md](dev/STATUS.md).

**Later**
- Video and audio sealing:
  - Video: per-segment hashes plus keyframe fingerprints, sealed as a C2PA manifest per chunk.
  - Audio: an audio fingerprint plus segment hashes.
- Recipient-only case links: the recipient signs in with Internet Identity, and keys come from vetKeys
- Safety mode: app lock and location stripping
- Play Integrity, through a separate verifier whose result is labeled as such
- Creator profiles and *first seen* leaderboards for viral originals
- Teams workspaces and the Verify API (web)
- Witness co-signing: two phones at the same scene cross-sign each other's captures
- Screen-recapture detection improvements

## 11. Business model

### 11.1 Keep costs tiny by design
ICP storage costs roughly **$5 per GB per year**, so what we store decides what we spend.

| Stored on-chain | Size | Cost |
|---|---|---|
| Permanent proof record: hashes, commitments, watermark ID, key ID, timestamps | ~1 KB | ~1M captures ≈ 1 GB ≈ **$5/yr** |
| Erasable fingerprint index: ~17 PDQ shapes, region grid, lookup tables | ~2.3 KB (to be measured) | ~1M captures ≈ 2.3 GB ≈ $12/yr |
| Anchoring | Each upload batch extends a certified append-only log | Effectively zero per capture |
| Proof-page preview (redacted JPEG ≤ 150 KB) and link-preview image (≤ 60 KB) | ~210 KB | ~1M posts ≈ 200 GB ≈ $1.1k/yr, erasable |
| Encrypted case payloads | Varies | Deleted at expiry |
| Full-resolution media | ~3 MB per photo | ~1M photos ≈ $16k/yr, so **paid tiers only** |

- **The default is to store proofs, not photos.** Originals stay on the phone, and posts are already hosted by the platforms.
- **Users never pay in or hold crypto.** Canisters pay for their own compute and storage. Revenue is converted to cycles off-platform, and canisters are topped up automatically.
- **Runway:** canisters hold 6–12 months of cycles in reserve, with a high freezing threshold so a missed top-up can never delete anyone's proofs.

### 11.2 Revenue
| Stream | Who pays | Sold where | Price idea |
|---|---|---|---|
| **Free** | Nobody | n/a | Unlimited sealed captures, anchoring and checking; proof pages with previews, rate-limited per device |
| **Witness Plus** | Individuals and creators | In-app via **Google Play Billing** (through RevenueCat, which handles receipt validation) | ~$2.99/mo or $24.99/yr |
| **Teams** | Insurers, adjusters, inspectors, property managers, newsrooms | Web (contract or card) | ~$10–15 per seat per month |
| **Verify API** | Newsrooms, trust & safety teams, insurers, platforms | Web | Per lookup or monthly; free for individual journalists |
| **Grants** | ICP ecosystem, press-freedom and digital-integrity funds | Off-platform | Early runway |

**Witness Plus** includes:
- full-resolution hosting for proof pages, with **25 GB included**. Hosted media stays while the subscription is active, plus a 30-day grace period with full export; proof records are permanent on every plan.
- video and audio sealing
- cases with PDF reports, and case links up to 256 MB (16 MB free)
- encrypted share links that expire
- backup and multi-device sync
- an opt-in creator profile

**The long-term engine is the Verify API.** Capture stays free and grows the registry; organizations that check content at scale pay for lookups. The more people capture, the more verification is worth.

**What we won't do:**
- ads
- selling data
- tokens, NFTs or "mint your photo"
- charging to verify a single image, because free public checking is what makes the network credible

## 12. Compliance

### 12.1 Google Play

| Area | Rule we follow |
|---|---|
| **Payments** | All in-app digital purchases (Plus) use **Play Billing**. No selling cycles or ICP in the app, and no crypto payments. |
| **Off-app sales** | Teams and API are sold on the web. The app only signs users in to accounts bought elsewhere, with no in-app links or buttons steering them to buy outside Play. |
| **Donations** | No in-app donation prompts. Supporting the project is handled on the website. |
| **Blockchain** | ICP is backend infrastructure only. No tokens, no earning mechanics and no tradeable assets in the app. |
| **Permissions** | Camera and fine location are requested only while the camera is open, with a clear explanation first. No background location. Gallery imports use the **Photo Picker**, not broad media access. |
| **Background work** | Anchoring uses **WorkManager**, with no foreground service. |
| **User data** | The Data safety form declares the uploaded hashes, fingerprints, region grids, watermark IDs, previews and encrypted case payloads, purchase data shared with RevenueCat, and opt-in crash reports and usage counts sent to Firebase (off by default). **Account deletion** is available both in the app and on the web; it removes hosted media and anything linking records to the person. The screen states plainly that anonymous proofs remain on-chain. |
| **AI-generated content** | AI output (captions, alt text, reports) is labeled, and users can report or flag it in the app. |
| **Claims** | Store listing and app copy say "verifies it was captured on a genuine device and hasn't been altered". We never say "court-admissible" or "proves it's true". |
| **Integrity** | Hardware Key Attestation is part of the proof. Play Integrity is not used in the MVP. |
| **Safety mode (later)** | An app lock only. No disguised icons or app impersonation, which Play prohibits. |
| **Subscriptions** | Clear pricing, trial terms and easy cancellation, following Play's subscription policy. |

### 12.2 Website

| Area | Rule we follow |
|---|---|
| **Privacy** | No cookies, analytics or third-party requests. Fonts are self-hosted, and Compose's default Google Fonts fallback is turned off. Checked images are processed in the browser and never uploaded. |
| **Private case links** | The access key lives in the URL `#fragment`, which browsers never send to a server. Case pages are marked `noindex`, and their link previews carry no case details. |
| **User-generated content** | Public proof pages have a report link, a takedown process and published contact information. Exact location is never shown publicly by default. |
| **Accessibility** | Each proof page's verdict is real HTML first, so it works with screen readers, translation and find-in-page. Inside the Compose view, every element has semantic labels. |
| **Payments** | No purchases on the website for app features. Teams and the Verify API are sold through their own web pages. |

## 13. Success criteria

- The demo scenario above runs end to end on a physical Pixel, and every web check works in Chrome, Firefox and Safari on desktop, Android and iPhone.
- **Matching benchmark.** Run Witness captures through Instagram, X, TikTok and WhatsApp as posts, screenshots and re-crops, searched against a ≥ 1 M-photo distractor index: **≥ 98% recall**, and **< 0.1% false matches** measured as a 95% upper bound over ≥ 5,000 negative queries. The test set and results are published in the repo.
- **Integrity is absolute for sealed files.** A modified sealed file is *always* *Altered*, and an untouched one is *always* *Original*. These are cryptographic checks, not statistical ones.
- **Local edits are caught.**
  - Where a reference image exists, a plate, sign or face changed in a re-saved copy is reported as *Altered* in **≥ 95%** of the benchmark's local-edit cases.
  - With the region grid only, edits of half a cell or more are caught, and smaller ones show "some regions couldn't be compared".
- **No false accusations.** Unedited copies (filters, crops, screenshots) are reported as *Altered* in **≤ 0.1%** of cases, measured as a 95% upper bound.
- **Same answer everywhere.** The app and the website give identical verdicts on the same input.
- The AI produces sensible blur suggestions, alt text and case reports with no network.
- Anchoring survives airplane mode, app restarts and reboots.
- A stranger with only a link sees the verdict summary **instantly**, with no wait for the Compose app. On a mid-range phone over 4G, checking a screenshot takes under 10 seconds on a first visit: fingerprints and Content Credentials first, with the watermark model loaded only if needed.
- Proof links show a correct preview (title, verdict, thumbnail) in Messages, WhatsApp, X and Slack.

## 14. Risks and open questions

- **Watermark robustness.** It has to survive each platform's compression and resizing, which needs testing against Instagram, X, TikTok and WhatsApp specifically.
- **Fingerprint search at scale.** A linear scan can't search millions of fingerprints inside one query. The index uses multi-index hashing (the 256-bit PDQ split into 16 chunks, one bucket table per chunk) in stable memory, sharded across canisters by record range and queried in one fan-out. See [ARCHITECTURE.md](ARCHITECTURE.md).
- **Screen recapture.** This is the hardest honest gap. It needs clear messaging and best-effort detection.
- **Device coverage.** StrongBox, attestation and raw satellite data vary by phone. Pixels are strong, but cheaper phones may only reach *Medium* confidence.
- **Built-in AI coverage.** Gemini Nano only runs on recent, higher-end phones. The no-model path has to be just as polished.
- **A2UI is alpha.** Google's A2UI Compose is at 1.0.0-alpha02, so expect API changes. It's only used in the Android app, never on the website.
- **Compose for the web is Beta.** Expect API changes and early-adopter bugs.
  - **Download size:** the web app is roughly 5 MB compressed, which is why every page shows real HTML first.
  - **Canvas rendering:** text drawn by Compose doesn't support find-in-page or translation like normal HTML.
  - **Browser back and forward:** Navigation 3 doesn't handle the browser history yet. The site's three routes are wired to the address bar by hand.
- **Watermark reading in the browser.** Adobe's TrustMark decoder model is about 47 MB, plus the onnxruntime-web runtime.
  - Checks run in stages: the fingerprint and Content Credentials first (a few MB), and the watermark model only if those find no match.
  - Both the model and the runtime are self-hosted on the web canister, so no third-party requests are made.
  - A smaller quantized decoder is worth evaluating.
- **One Content Credentials reader.** Witness verifies its own Content Credentials with a small reader in its Rust core, the same on the phone and the web. Credentials from other tools are shown as "other credentials present" and never affect a verdict.
- **iPhone users can't capture.** They can check and view everything on the website. Adding iOS later means extra platform code for capture and attestation, plus a decision on A2UI for iOS.
- **Play policy changes.** Rules on payments and links outside Play are shifting by region. We stay on Play Billing in-app regardless.
- **Name (decide before any permanent link).** "Witness" collides with WITNESS.org, a human-rights nonprofit working on media authentication. Renaming after watermark payloads, C2PA claim names and URLs ship is costly, so the brand and **custom domain** must be settled before the first public proof link. Raw `icp0.io` URLs are never used in shareable links.

## 15. Building blocks (light)

| Layer | Choice |
|---|---|
| Shared UI (Android + web) | Kotlin 2.4, **Compose Multiplatform** 1.12 (`androidTarget` and `wasmJs`, with automatic JavaScript fallback for older browsers), Metro for dependency injection, kotlinx-serialization. Verdict, proof and chain-of-custody composables are shared by both. |
| Android app | Navigation 3; Room 3 + DataStore; CameraX with Compose preview; Keystore + Key Attestation; raw GNSS; WorkManager; Gemini Nano via ML Kit; **A2UI Compose** (Google's own) with a fixed component catalog; RevenueCat for Play Billing; Firebase Analytics + Crashlytics, opt-in and only when a Firebase config file is present |
| Website | Compose Multiplatform for the web, reading the URL path at startup for its 3 routes. JavaScript bridges, called from Kotlin: `@icp-sdk/core` for canister calls (with certificate and proof checks on every read), the Rust core compiled to WebAssembly for fingerprints, Content Credentials and verdicts, and later onnxruntime-web with TrustMark's decoder for the watermark (loaded only when needed). No telemetry. |
| Rust, shared | `witness-core`: plain Rust with no platform bindings, a facade over small crates for the proof format, image decoding, PDQ fingerprinting (full frame and standard crops), the region grid, alignment and matching, Content Credentials (writing and reading), Merkle proofs, attestation checks, the watermark payload, and the verdict rules. It is used by the app, the website and the canisters, and gives bit-identical results on phones and in the browser. |
| Rust, Android | `witness-android`: capture sealing and Content Credentials through `witness-core`, ICP calls (ic-agent), and the TrustMark model runner (optional), bound to Kotlin with plain UniFFI. Hardware keys stay in Kotlin and are called back from Rust. |
| Rust, web | `witness-web`: the check, certificate and proof verification, and case decryption compiled to WebAssembly with wasm-bindgen |
| Backend (Rust canisters) | Split by job so no single canister becomes the bottleneck. **Registry:** permanent minimal records in a certified append-only log; Key Attestation verification with pinned Google roots and a revocation list synced by HTTPS outcall; the Witness C2PA certificate authority (threshold Ed25519); the shard map. **Index shards:** fingerprints, region grids, multi-index-hashing PDQ buckets, watermark and exact-hash tables, all erasable; clients query every shard directly. **Pages:** certified HTML pages for `/p/` and `/v/`. **Media shards:** previews, link-preview images and encrypted case payloads, all erasable. **Web:** the static Compose bundle (WebAssembly plus a JavaScript fallback), fonts and the watermark model. |
| Identity | Optional Internet Identity |
| Local development | The `icp` CLI deploys all five canisters to a local replica with one script, and a smoke test runs the whole flow against it |

## 16. Project layout

```
witness/
  client/                Gradle root for the Compose Multiplatform client
    build-logic/         Convention plugins
    native/witness/      The one Android module carrying the Rust library and its UniFFI bindings
    core/                model · designsystem · ui · domain · verify · icp (Android + web) · data · platform · a2ui (Android)
    feature/             home · capture · post · case · share · settings (Android) · check · proof (Android + web)
    androidApp/          Android app shell
    webApp/              Web entry point, index.html and the JavaScript bridges
  crates/                Rust workspace
    witness-core/        Facade over the pure crates (format, image, fingerprint, match, c2pa, merkle, attest, watermark, verdict): no platform bindings
    witness-android/     Sealing, ICP calls, optional TrustMark runner; UniFFI bindings for Android
    witness-web/         WebAssembly build of witness-core for the browser (wasm-bindgen)
  canisters/             common · registry · index · pages · media · web
  tools/witness-smoke/   End-to-end smoke test against the local replica
  scripts/               Local deploy, smoke, native and canister builds, web staging, model fetch, dependency checks
  design/                UI design: witness.figcraft, renders/, brand/, sample photos, and the tools that generate them
  docs/                  SPEC.md (product), ARCHITECTURE.md (technical design), dev/ (build reports and STATUS.md)
```

`witness-core` must never depend on wasm-bindgen or the `c2pa` crate, because either one would break the canister build. `scripts/check-core-deps.sh` and `scripts/check-canister-deps.sh` check for this. Developer setup and commands are in the root `README.md`.

Regenerate the design with `zsh design/tools/build.sh` (add `FILTER=B` to render one flow). To regenerate the sample photos, run `zsh design/tools/images.sh`.
