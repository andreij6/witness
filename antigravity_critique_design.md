# Design Review & Critique: Witness (`design/`)

**Reviewed artifacts:**
- Rendered frames: `design/renders/00-foundations.png` through `11-altered.png` and `overview.png`
- Design generator & tokens: [gen.py](file:///Users/andrejones/Desktop/workspace/projects/witness/design/tools/gen.py)
- Brand icon: [icon.svg](file:///Users/andrejones/Desktop/workspace/projects/witness/design/brand/icon.svg)
- Product spec alignment: [SPEC.md](file:///Users/andrejones/Desktop/workspace/projects/witness/docs/SPEC.md)

---

## Executive Summary

The Witness design system has **strong visual identity and remarkably clear storytelling**. The warm paper surfaces (`#F4F3EF` / `#FFFFFF`), ink typography (`#15171B`), dark camera canvas (`#0B0C0E`), disciplined green-only-when-proven rule (`#0A8754`), and violet A2UI surface language (`#7A3FF2` rail + hairline border + `✦` tag) make a complex cryptographic product feel calm, legible, and trustworthy without falling into generic "web3/cyber" visual clichés.

The main issues fall into four buckets:
1. **Accessibility & semantic color clashes:** `MUTED` (`#8A8F98`) fails WCAG AA contrast on light paper/white across dozens of labels; `#2D3BE6` (`PRIMARY`) does double duty as both interactive CTA blue and the `"Matches, with changes" / "Declared edit"` verdict color.
2. **Layout & component defects in the renders:** Top telemetry pills overflow/clip at the right edge of `02-camera-post` and `06-camera-case`; `Cases` card layout alignment in `01-home` has an uneven gap for single-thumbnail cases; the bottom `Capture` FAB overlaps the bottom edge of the second case card.
3. **Spec-to-UI mismatches:** Compliance-mandated controls (AI output flagging/reporting, public proof-page report/takedown links, `#fragment` key indication, and "what we don't prove" on `/p/<id>`) are missing from the corresponding screens, while `09-share-case`'s `"Include AI summary"` toggle disappears in `10-case-link`.
4. **Missing critical states & flows:** No `No Witness record` screen, no `Check` tab landing/input state, no desktop web layout (despite Demo Step 6 explicitly opening on a laptop), no case-selector on `Camera · Case`, and no `Witness Plus` / `Safety mode` / `Unverified import` views.

---

## 1. Foundations & Design System (`00-foundations.png` & `gen.py`)

### 1.1 Strengths
- **Editorial trust aesthetic:** `PAPER` (`#F4F3EF`) + `SURF` (`#FFFFFF`) + `LINE` (`#E0DCD2`) reads like an archival ledger/document rather than a sterile utility or crypto wallet.
- **5 clear Design Principles:** Stating *"Green means proven — never decoration"* and *"AI suggests, the person decides — and can see which is which"* creates concrete visual rules that hold up well across most screens.
- **Distinct AI boundary (`generated_surface`):** The 1.2px `#7A3FF2` border (35% alpha) + 3px left violet rail + `auto_awesome` tag is immediately recognizable and cleanly separates deterministic proof data from probabilistic on-device AI suggestions.
- **App icon (`icon.svg`):** The camera-ring + shutter button + emerald checkmark (`#34D399` on `#15171B`) scales well down to 48px and adapts cleanly to Android themed monochrome (`#1E2A8A` on `#DDE3FF`).

### 1.2 Issues & Recommendations
1. **WCAG AA contrast failures (`MUTED`, tinted `OK` / `WARN` chips):**
   - `MUTED` (`#8A8F98`) on `SURF` (`#FFFFFF`) is ~3.2:1 and on `PAPER` (`#F4F3EF`) is ~2.9:1. It is used extensively for 11–12.5px text (`STILL RECOMMENDED`, `WHAT CHANGED`, `LINK EXPIRES`, `"Anyone can check this post..."`, `"Served from an ICP canister · certified response"`, `"Doesn't prove the scene wasn't staged"`). **Fix:** Darken `MUTED` to at least `#666B75` (~4.8:1 on `#F4F3EF`, ~5.4:1 on `#FFFFFF`) for readable small text.
   - `OK` (`#0A8754`) on 12% alpha `OK` chip backgrounds (`~#E2F0EA`) is ~3.9:1, and `WARN` (`#B4690E`) on 12% alpha `WARN` (`~#F6EDE2`) is ~3.8:1—slightly under 4.5:1 for 11–12px chip labels (`Anchored`, `Queued`, `Checked 1,284×`). **Fix:** Darken chip foreground text slightly (e.g. `#076E44` for `OK_TEXT`, `#925309` for `WARN_TEXT`) while keeping `#0A8754` / `#B4690E` for icons/fills.
2. **Semantic overloading of `PRIMARY` (`#2D3BE6`):**
   - In `00-foundations.png`, `#2D3BE6` is named `action` (buttons, links, switches, selected photo checkboxes), **yet** it is also the semantic status color for **`Matches, with changes`** (`04-check`) and **`Declared edit`** (`00-foundations`, `09-share-case`, `10-case-link`).
   - On `09-share-case.png`, blue selected checkboxes, blue active `"Blur faces · 1"` chip, blue `"Edited"` badge on Photo 4, and the blue `"Share link"` CTA all share `#2D3BE6`, conflating *selection/action* with *modification state*.
   - **Fix:** Assign a distinct color for `EDITED / MATCHES_WITH_CHANGES` (e.g., deep teal `#0E7490` or slate-indigo `#3B5BDB` / warm cobalt distinct from interactive CTA blue, or move interactive action blue to `#1D4ED8` and edited-status to `#0F766E`).
3. **Undocumented dark-mode greens:**
   - `00-foundations.png` only lists `proven: #0A8754`, while `gen.py` also uses `#34D399` in the app icon (`icon.svg` / `mark()`) and `#4ADE80` in camera HUD pills (`Sealing · StrongBox`, `Location · High`). Add `proven-dark` (`#34D399` / `#4ADE80`) explicitly to the Foundations palette.
4. **Typography & Compose Multiplatform web constraint:**
   - Foundations specifies `Inter` and `Menlo` (`gen.py` line 36). Note that `Menlo` is a macOS/iOS system font—it does **not** ship on Android or Windows browsers! In Compose (`SPEC.md §12.2`: *"Fonts are self-hosted"*), bundle an open-source monospace font such as `JetBrains Mono` or `IBM Plex Mono` alongside `Inter` so Android, Linux, Windows, and Web render identically.

---

## 2. Screen-by-Screen Review

### `01 · Home` (`01-home.png`)
- **Strengths:** Fast scan hierarchy (trust summary line → `All / Posts / Cases` chips → hero post with `"Checked 1,284×"` → 2-up grid → Cases list → center `Capture` FAB). Strongly supports *"Camera first"*.
- **Issues:**
  1. **Hardcoded thumbnail offset in Cases (`gen.py` lines 738–742):** `"Fender bender"` has 3 stacked thumbnails (`x = 28..104`) and `"Kitchen ceiling leak"` has 1 thumbnail (`x = 28..84`), but the folder icon starts at `M + 96` (`x = 112`) for both, creating an uneven 28px dead gap on `"Kitchen ceiling leak"`. Either left-align relative to thumbnail stack width or render a fixed-width 3-slot/collage container.
  2. **Bottom FAB overlaps the last case card:** The raised `Capture` halo (`y = 813` top of dark circle, halo even higher) clips into the bottom border of `"Kitchen ceiling leak"` (`y = 774..848`). Ensure the scrollable list has bottom clearance above the elevated FAB, or flatten the FAB into the bar.
  3. **Missing empty / unverified / media-type states:** There is no visual distinction for **gallery imports (`Unverified`)**, **failed/expired** items, or **video/audio** badges. Also, tapping `See all` or creating a new Case has no visible `+ New case` affordance from Home.

### `02 · Camera · Post` (`02-camera-post.png`) & `06 · Camera · Case` (`06-camera-case.png`)
- **Strengths:** Dark, high-contrast viewfinder; `Post | Case` mode switch at top; live StrongBox + GNSS telemetry; tactile shutter + bottom GNSS verification line; crisp dark violet `Evidence coach` card on `06`.
- **Issues:**
  1. **Top telemetry pill row overflows the right viewport edge:** On both `02-camera-post.png` and `06-camera-case.png`, the 3 horizontal pills (`Sealing · StrongBox`, `Location · High`, `4:12:09 PM` / `2:32:07 PM`) start at `x = 16` and push the third pill right against/past `x = 412` (`PM` touches the right edge with 0px margin). On smaller 360dp Android screens or longer locales, this will clip off-screen. **Fix:** Use 2 compact rows, shorten labels (`StrongBox` / `Location: High`), or drop the clock pill (the status bar already shows the time, or place a compact timestamp inside the shutter/meta strip).
  2. **No active-case selector in `Camera · Case` (`06-camera-case.png`):** Switching from `Post` to `Case` jumps straight into `EVIDENCE COACH · FENDER BENDER (3 of 5)`, with no UI control to switch to `"Kitchen ceiling leak"` or start a **New Case**. Without an active case dropdown/chip, users will accidentally capture evidence into the wrong case.
  3. **HUD clutter vs. "the scene is the hero":** Between 4 white corner brackets, 2 yellow bounding boxes + yellow pill tags (`House number · auto-blur`, `Plate · auto-blur`), 3 top dark pills, and the dark bottom AI banner, a large portion of the viewfinder is covered. Consider making the corner brackets subtler or removing them, and making the yellow auto-blur pills interactive (tap to toggle blur on/off directly in viewfinder, or keep boxes minimal until captured).
  4. **Yellow `#FFD43B` reuse:** `#FFD43B` is used for both the active `PHOTO` mode label and `auto-blur` bounding boxes, creating an unintended visual link.
  5. **`VIDEO` and `AUDIO` modes in MVP:** Both camera screens display `VIDEO | PHOTO | AUDIO`, even though `SPEC.md §10` defers video and audio sealing to **Later** (and §11.2 paywalls them under **Witness Plus**). Either hide them for MVP or show a clear lock/Plus/coming-soon state; additionally, `AUDIO` capture has no non-camera viewfinder design.
  6. **Thumbnail tap target inconsistency:** In `gen.py` line 805, tapping the bottom-left thumbnail in `02-camera-post` navigates to `05-proof-page` (external web view) rather than an in-app item/post view, whereas `06-camera-case` navigates to `08-item-proof`.

### `03 · Post composer` (`03-post-composer.png`)
- **Strengths:** Clean `Edit` affordance on photo, transparent `"BEFORE YOU POST"` A2UI card, clear proof link preview (`witness.icp0.io/p/H4IL-7QK`), and `"Add proof link to caption"` toggle.
- **Issues:**
  1. **Action mismatch (`"tap to undo either"` vs. single `"Undo"`):** The first row says `"Blurred house number & a plate"` / `"Declared edits · tap to undo either"`, but only offers one `"Undo"` link on the right—making it impossible to undo just one without an extra unpictured modal. Change to `"Review"` / `"Edit"` or list individual chips (`House number ×`, `Plate ×`).
  2. **`CAPTION` looks read-only:** Unlike `"DRAFT SUMMARY"` on `07-case` (which has an `Edit` action), the Caption box has no `Edit` button, cursor, or input border to signal editability.
  3. **Missing AI report/flag affordance (`SPEC.md §12.1`):** The spec mandates users can *"report or flag [AI output] in the app"*, yet neither `03-post-composer` nor `07-case` has a flag/thumbs-down/feedback icon on any AI-generated card.
  4. **Redacted `"Original"` badge ambiguity:** The badge on the blurred photo says `Original · 2 declared blurs` in black, while `05-proof-page` uses a green `Original` pill. Be careful that a green `Original` pill on an edited/blurred photo doesn't confuse viewers who associate green `Original` strictly with *"Matches the sealed capture exactly"* (`SPEC.md §4.8`).

### `04 · Check` (`04-check.png`)
- **Strengths:** The side-by-side comparison showing the Instagram screenshot (`@stormwatch.den`, `1.9k likes`) next to the `"SEALED ORIGINAL"` with the dashed blue `Crop` overlay is an outstanding piece of explanatory UX. `"WHAT CHANGED"` translates technical verification into plain English.
- **Issues:**
  1. **Logical contradiction on `"SEALED ORIGINAL"`:** Look at the right-hand photo in `04-check.png` (`gen.py` line 909-910): it renders the **blurred** image (house number & plate already mosaicked) while `"WHAT CHANGED"` lists `"2 blurs declared by the photographer"`. If the right image already has the blurs applied, it isn't the unedited original; conversely, if the photographer blurred PII for privacy, the public check screen *must not* show the unblurred original. Label the right pane `"Sealed post (with 2 declared blurs)"` or separate `"Sealed original"` from `"Privacy redactions (hidden)"`.
  2. **Missing Check empty/input state:** `Home` has a `Check` bottom tab, but `04-check.png` only shows the `"Shared from another app"` result state. There is no screen showing how a user picks a file, pastes a link, or views recent checks when tapping the `Check` tab.
  3. **Verdict label inconsistency:** `SPEC.md §4.8` says `Matches an original, with changes`, `00-foundations.png` says `Matches, with changes`, and `04-check.png` renders `Matches an original` (21pt) over `with changes` (14pt subtitle). Standardize the exact copy across spec, foundations, and UI.
  4. **No zoom / compare-loupe affordance:** At ~166dp wide, small edits (like a changed plate digit or blurred face) are hard to inspect. Adding a tap-to-zoom / swipe-diff hint fulfills Principle 2.

### `05 · Proof page (web)` (`05-proof-page.png`)
- **Strengths:** Crisp mobile-first browser layout, clear summary card (`Captured`, `Location`, `Device`, `Anchored`), `"First seen here · checked 1,284 times · anonymous"`, and ICP certified response footer.
- **Issues:**
  1. **Missing mandatory compliance & trust disclosures:**
     - `SPEC.md §12.2` requires *"a report link, a takedown process and published contact information"* on public proof pages—none are present on `05-proof-page.png`.
     - Unlike `10-case-link.png`, `05-proof-page.png` omits the **"What this proves / Doesn't prove"** card, violating Principle 3 (*"Say what we don't prove as clearly as what we do"*).
  2. **No drill-down to cryptographic evidence:** `Device: Genuine Pixel 9 Pro · attested` and `Anchored: 4:13 PM · Internet Computer` have no tap/expand affordance to inspect Key Attestation checks, C2PA manifest, or certificate details (Principle 2).
  3. **Desktop vs. Mobile web copy:** `"Drop a screenshot or paste a link"` doesn't fit a mobile touch browser (`05-proof-page` is 412×915)—use `"Choose a screenshot or paste a link"` on mobile, and design a desktop responsive layout too.

### `07 · Case` (`07-case.png`)
- **Strengths:** Best-in-class mobile evidence workspace. The AI checklist (`STILL RECOMMENDED`) with direct `"Capture"` action, mini car impact diagram, OCR-filled plate (`7MNV562 · ✦ Read from photo 3`), and `"Anyone injured?"` control feel fast and practical in a stressful situation.
- **Issues:**
  1. **Missing AI styling on `"DRAFT SUMMARY"`:** `SPEC.md §4.6` says the AI *"Drafts a neutral summary you can edit"* and *"Anything the AI generated is visibly marked."* On `07-case.png`, `"DRAFT SUMMARY"` uses a plain white card with no violet border/rail and no `✦` tag—inconsistent with `03-post-composer`'s `✦ Suggested` caption card and `00-foundations.png`.
  2. **No dismiss/remove control on AI blocks:** `00-foundations.png` promises users *"can change or remove"* AI-built report parts, and `§12.1` requires flagging bad AI output, neither of which is visible on the checklist or Accident details cards.
  3. **Right-edge clipping of the `+` tile:** In the horizontal Evidence strip, the dashed `+` card at `x = 384` only shows its left 28px and the `+` icon sits near the frame edge. Giving it a slightly tighter card width or visible scroll fade would look more intentional.
  4. **Unclear `Export` behavior:** Does `Export` produce a PDF (paywalled under Witness Plus in `§11.2`) or a raw ZIP/JSON bundle? Worth clarifying via tooltip/subtitle or sheet.
  5. **Demo asset continuity:** Across `06-camera-case`, `07-case`, `08-item-proof`, `10-case-link`, and `11-altered`, the 4 accident photos depict different car models and mismatched license plates (`8KTR214`, `7MNV562`, `80D4022`, `CV82150`). Regenerating cohesive photos for the demo case would make pitch/portfolio walkthroughs much more convincing.

### `08 · Item proof` (`08-item-proof.png`)
- **Strengths:** Clean bottom-sheet over full-bleed media, clear 4-step vertical timeline (`Captured` → `Sealed` → `Location verified` → `Anchored`) with monospace timestamps and clear metadata.
- **Issues:**
  1. **Missing check-strength breakdown (`SPEC.md §4.2`):** The spec promises *"Every proof lists which checks ran on that phone and how strong each was."* Here, Play Integrity and Key Attestation chain strength (StrongBox vs. TEE) are compressed into a single line (`StrongBox key · C2PA · sha256...`), and `"View record"` isn't explained (raw JSON vs. web proof page?).
  2. **No case/post context:** When opened from search or Home, the sheet doesn't show which Case (`Fender bender`) or Post this item belongs to, or provide a quick `"Move to case / Create post"` action (`SPEC.md §4.1`).

### `09 · Share case` (`09-share-case.png`)
- **Strengths:** Very thorough sharing flow: multi-select thumbnails, instant privacy chips, expiry selector, QR + copyable link, and a `"RECIPIENTS WILL SEE"` live summary before sending.
- **Issues:**
  1. **Link omits `#fragment` encryption indicator (`SPEC.md §6.3, §12.2`):** The spec emphasizes that private case link keys live in `#fragment` so the server never sees them, yet both the displayed URL (`witness.icp0.io/v/8F2K-QM7D`) and `10-case-link.png`'s address bar show no `#fragment`. Show a masked fragment or explicit E2EE indicator (`End-to-end encrypted · key stays in link`) so security claims and UI match.
  2. **Toggle disconnect with `10-case-link.png`:** `"Include AI summary"` is toggled **ON** in `09-share-case.png`, yet `Preview ->` (`10-case-link.png`) displays **no AI summary, no incident details, and no case title**.
  3. **Tight footer padding inside QR card:** `"Preview ->"` at `y = 793` sits slightly cramped inside the bottom of the card (`y = 544..818`).

### `10 · Case link (web)` (`10-case-link.png`)
- **Strengths:** Excellent recipient summary with `"WHAT THIS PROVES"` (including `— Doesn't prove the scene wasn't staged`) and per-item status icons.
- **Issues:**
  1. **Rows look non-interactive:** Photos 1–4 have status icons on the right (`✓` / `✏️`) with no chevron or tap cue, so a recipient (e.g., an insurance adjuster) cannot tell if they can open full-size photos, inspect chain-of-custody per item, or download the case package.
  2. **Missing case header & AI summary:** No title (`"Fender bender"`), no `"Include AI summary"` content (contradicting `09-share-case`), and no indication of whether location was hidden/redacted by the sender.
  3. **Missing report/legal footer (`SPEC.md §12.2`).**

### `11 · Altered (web)` (`11-altered.png`)
- **Strengths:** Immediately alarming without being melodramatic; highlighting the forged plate `7MNV862` vs `Original · Nov 18` (`7MNV562`) makes tampering tangible in one glance.
- **Issues:**
  1. **Dead-end screen:** Unlike `05-proof-page` and `10-case-link`, `11-altered.png` has **no bottom action**—no `"Check another file"` drop zone, no `"Report"`, and no `"What this proves"` card.
  2. **Technical accuracy of `"Seal: Broken — pixels changed in 1 region"`:** As noted in the spec critique, a re-saved or screenshotted forgery won't have a "broken seal" inside the file; it matched via perceptual fingerprint/watermark and differs from the reference. Saying `"Content: Differs from sealed original (1 region)"` is more accurate.

---

## 3. Missing Screens & Flows to Design Next

To cover the full MVP spec (`SPEC.md §8, §9, §10`) and mobile/desktop web targets, the design set needs:
1. **`No Witness record` verdict screen** (4th core verdict in `00-foundations` and Step 3 of the 2-minute demo).
2. **`Check` tab empty/input screen** (reached from Home's bottom nav `Check` tab and `witness.icp0.io/check` before dropping a file or pasting a URL).
3. **Desktop web responsive layouts (`~1280px`)** for `/p/<id>`, `/v/<id>`, and `/check` (since Demo Step 6 opens a case link on a laptop, and web drag-and-drop is primarily a desktop behavior).
4. **`Witness Plus` & Account / Settings / Safety Mode screens** (Play Billing purchase flow is in MVP §10; Safety mode app lock / quick-hide is in §4.9).
5. **Unverified gallery import state & Video/Audio capture states** (or explicit removal/gating of `VIDEO` and `AUDIO` from the MVP camera dial).
6. **AI feedback / flag dialog & Case switcher sheet** (for flagging bad Gemini Nano output per §12.1 and picking/creating cases in `Camera · Case`).

---

## 4. Top Quick Wins in `design/tools/gen.py`

If updating the FigCraft design next:
1. Darken `MUTED` from `#8A8F98` to `#666B75` (fixes WCAG AA across all 11 screens in one line).
2. Separate the `Matches, with changes / Declared edit` color from `PRIMARY` (`#2D3BE6`) action blue.
3. Fix the top telemetry pill row in `camera()` (`gen.py` lines 778–784) so the 3rd pill never clips at `x = 412`.
4. Fix the `Cases` thumbnail offset in `01-home` (`gen.py` line 741) and add bottom padding above the `Capture` FAB.
5. Add A2UI violet framing + `✦` tag to `DRAFT SUMMARY` on `07-case`, add AI flag icons, add `#fragment` indicator + AI summary on `09`/`10`, and add `"What this proves"` + Report links on `05`/`10`/`11`.
