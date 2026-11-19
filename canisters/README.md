# Canisters

| Crate | Package | Deployed as | Role |
|---|---|---|---|
| `common/` | `witness-canister-common` | (library) | Shared Candid types, stable-memory layout, runtime helpers |
| `registry/` | `witness-registry` | `registry` | Permanent records, keys, MMR/SMT, erasure coordinator |
| `index/` | `witness-index` | `index_0` | PDQ/watermark/exact-hash index shard |
| `pages/` | `witness-pages` | `pages` | Certified proof and case shells |
| `media/` | `witness-media` | `media_0` | Preview, og, full-resolution and case blobs |
| `web/` | (static-site recipe) | `web` | Serves `client/webApp` (or `web/dist-placeholder`) |

Canister crates are `cdylib` + `rlib`, built for `wasm32-unknown-unknown` with ic-cdk 0.20.3, ic-cdk-timers 1.0, ic-stable-structures 0.7.2 and candid 0.10. `registry` has the cargo feature `dev-attestation` (local builds only), which turns on `witness-attest/dev-attestation`.

## The shared contract (`common/`)
- `common/common.did` is the Candid source of truth for every shared type. `tests/did.rs` fails if a Rust type and the `.did` drift apart, or if a `.did` type has no Rust twin.
- Init arguments: `registry` takes `RegistryInit { index; media; pages }`. `index`, `pages` and `media` take `PeerInit { registry; shard; origins : opt Origins }` (`pages` ignores `shard`; `origins` sets the public site, case, app, api and canister origins, defaulting to production, and `http` origins need the `local-dev` feature). `post_upgrade` takes the same argument.
- `runtime` holds the IC-only helpers: `set_certified_root`, `certificate`, `inspect` (for `canister_inspect_message`), `start_timers` (call it from both `init` and `post_upgrade`), `timer_statuses` and `status`.

## Stable memory ids (`common::memory`)
Every canister has its own `MemoryManager`, reached through `memory(id)`.

| Id | registry | index | pages | media |
|---|---|---|---|---|
| 0 | `CONFIG` (init args cell) | `CONFIG` | `CONFIG` | `CONFIG` |
| 1–7 | reserved (common) | reserved | reserved | reserved |
| 8 | `MMR_NODES` | `PDQ_ROWS` | `CERT_TREE` | `BLOBS` |
| 9 | `MMR_PEAKS` | `GRIDS` | `SHELLS` | `BLOB_INDEX` |
| 10 | `SMT_NODES` | `POSTINGS` | `STATE` | `UPLOADS` |
| 11 | `RECORDS` | `WATERMARK` | | `QUOTAS` |
| 12 | `RECORD_BY_ID` | `EXACT_ASSET` | | `GRANT_USAGE` |
| 13 | `KEYS` | `EXACT_PIXEL` | | `EXPIRY` |
| 14 | `PENDING_KEYS` | `DERIVATIVES` | | `CERT_TREE` |
| 15 | `COUNTERS` | `LOCAL_TO_RECORD` | | `STATE` |
| 16 | `BY_CAPTURE` | `RECORD_TO_LOCAL` | | |
| 17 | `DERIVATIVES` | `STATE` | | |
| 18 | `TOMBSTONES` | | | |
| 19 | `OUTBOX` | | | |
| 20 | `QUOTAS` | | | |
| 21 | `REVOCATIONS` | | | |
| 22 | `II_LINKS` | | | |
| 23 | `STATE` | | | |
| 24–31 | spare (add in `common`) | spare | spare | spare |
| 32–254 | `memory::local(n)`, canister-private | same | same | same |

Ids are never reused or renumbered once deployed.

## Test
```sh
cargo test -p witness-canister-common
cargo clippy -p witness-canister-common -p witness-registry -p witness-index -p witness-pages -p witness-media --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown --release -p witness-registry -p witness-index -p witness-pages -p witness-media
```
PocketIC (`pocket-ic` 16.1, a dev-dependency of each canister) downloads its server binary on first use.

## Deploy (local replica only)
```sh
scripts/deploy-local.sh   # starts the local network on 127.0.0.1:8090 if needed, creates, builds and installs everything
scripts/stop-local.sh
```
- `icp.yaml` (icp CLI 1.7) defines the canisters, the network and the `local` environment.
- `scripts/build-canister.sh <package> [features]` runs `cargo build --release`, `candid-extractor`, `ic-wasm shrink` and adds the `candid:service` metadata. Its output goes to `target/canisters/`.
- `scripts/stage-web.sh` copies `client/webApp/build/dist/wasmJs/productionExecutable` (override it with `WITNESS_WEB_DIST`), or `web/dist-placeholder` when that has no `index.html`, into `web/dist`, and adds `web/_redirects` (§7.6) if it is missing.
- Canister ids and URLs are written to `.icp-local/ids.json`, which is git-ignored.
