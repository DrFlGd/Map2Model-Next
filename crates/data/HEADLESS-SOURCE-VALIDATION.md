# A2-02 — Native offline terrain CLI handoff (synthetic-first)

**Status:** Implemented in stacked draft PR #16. This is a **developer-only native tool**, not Electron IPC or a user-ready source acquisition service. The frozen G0 contracts are untouched. No network access or automatic downloads exist in the binary.

## Commands

### 1. Inspect an externally acquired local archive without authorizing it

```sh
cargo run --offline -p map2model-data --bin terrain_offline -- inspect ./interlaken-private.pmtiles

# Where z/x/y are independently verified actual indexed tiles:
cargo run --offline -p map2model-data --bin terrain_offline -- inspect ./interlaken-private.pmtiles 13 4275 2891
```

The second command selects one actual indexed tile and decompresses/decodes its **512×512 WebP**, returning the signed Terrarium north-west, center and south-east pixel heights, masked pixel count and encoded tile length. This format-only inspection requires the current bounded PMTiles v3 subset (≤128 MiB, ≤200,000 indexed tiles, WebP, directory modes none/gzip), **but permits real attribution-only metadata with no explicit `encoding` field**; it reports `metadataExplicitlyDeclaresTerrarium=false`, does not authorize source encoding, and retains all production-ready flags false. **Production registration/decode** still rejects absent explicit trusted Terrarium evidence. An authentic extract outside the supported subset should be reported as incompatible, not silently substituted or interpreted.

Output reports SHA-256 over the *actual local archive bytes*, archive length, header bbox/zooms and counted tile entries, compression modes and observed pixel values. It **always** sets `productionGenerationReady=false`, `verticalDatumVerified=false`, `sourceLicenseVerified=false`, and `nativeResolutionVerified=false`. An observed elevation number is not proof of vertical reference or lawful derivative rights.

### 2. Decode explicitly declared and SHA-pinned local source into frozen grid JSON

```sh
cargo run --offline -p map2model-data --bin terrain_offline -- decode \
  ./synthetic-local.pmtiles ./private-native-registry \
  ./synthetic-source-declaration.json ./selection.json 12 ./grid-v1.json
```

Input `synthetic-source-declaration.json` must be a strict local JSON object with camelCase equivalents of `SourceDeclaration`'s sourceName, releaseId, archiveId, **expectedArchiveSha256** (mandatory in this CLI), verticalDatumStatus/id, licenseIdentifier, attribution, nominalGroundSampleMeters, nativeResolutionSource, uncertaintyMeters and decoderVersion. `selection.json` must be a valid *frozen* `SelectionV1`, including canonical selectionId. Unknown datum rejects with `UNSUPPORTED_DATUM`; absent pin, wrong checksum or rights declaration rejects. No output is produced for a failed decode. The grid path must **not already exist**, avoiding silent overwrite.

The resulting `grid-v1.json` contains the full frozen `ElevationGridV1` with north-up, f64 row-major samples and a canonical SHA-256 that survives JSON round-tripping (Rust `serde_json/float_roundtrip` enabled in this geodata crate). The stdout summary deliberately states `licenseAndDatumIndependentlyVerified=false` and `productionGenerationReady=false`: the CLI cannot legally or geodetically authenticate arbitrary text declarations.

**Security contract:** This developer CLI accepts explicit local paths. It MUST NOT be exposed directly to the renderer or used to bypass Agent 1's native IPC/authorized registry controls. It registers only content-addressed native assets. No UI "Generation Ready" state is issued.

## Contract-compatible smoke and evidence

Run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p map2model-data --test offline_cli
```

`tests/offline_cli.rs` creates its own fully artificial PMTiles v3 z0 archive with an actual **512×512 lossless WebP** raster (zero-meter RGB Terrarium). It executes the compiled native command in a child process with HTTP proxies set to a loopback refusal endpoint; verifies format-only unknown-datum/unverified-rights report, byte SHA, raster samples and masked pixels, and explicit source SHA pin. It performs a second process invocation to generate `grid-v1.json`, reads the exact JSON back as `ElevationGridV1`, validates its *unchanged canonical hash* and source/selection linkage, and passes that very grid object into **Agent 3's merged real `map2model-mesh::build_mesh`**, requiring a closed, oriented, nonempty printable geometry report and canonical mesh SHA. It also tests no-clobber, missing SHA pin and unknown datum rejection. No fabricated/third-party terrain files are added to the repository.

## Privately supplied real Interlaken extraction — observed format evidence

The owner provided an actual 3,127,295-byte `interlaken-test.pmtiles` (Interlaken candidate bbox `7.88,46.68,7.89,46.69`) for private analysis. SHA-256 computed independently from the uploaded bytes: `40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce` — identical to the owner's PowerShell checksum. This **real third-party archive is NOT committed** to the public repository.

A separate bounded, read-only PMTiles v3 parser observed a gzip-compressed root directory containing 40 real index entries with z13=1, z14=2, z15=4, z16=9, z17=24. The 40 referenced WebP bytes decoded successfully as **512×512 RGB images** in Pillow's WebP decoder; no alpha-masked pixels were found. Decoding RGB using the independently documented Terrarium formula produced signed meter *values*, but **not independently certified vertical elevations or source permissions**. Example of format-only evidence (z13 x4275 y2891): RGB-decoded center represents 563.75 on the Terrarium scale. A real native Rust binary run on this file is still required when the private archive is accessible to the Windows developer CLI environment.

Two observed incompatibilities with the original strict importer policy:

1. The real metadata object contains **attribution only** and no `encoding` field. New `inspect_unverified_local_pmtiles` accepts this *format-only* and reports `metadataExplicitlyDeclaresTerrarium=false`; registered `import_local_pmtiles` / registry reopen still **reject** missing explicit Terrarium declaration. This is deliberate: RGB/WebP dimensions alone cannot prove the source height encoding or datum. A future explicit, evidence-backed source encoding policy is needed before registering such extracts.
2. The file's header says `minZoom=13,maxZoom=18`, but its actual indexed tile range is only **13–17**. The owner's official `pmtiles verify` reports `header MaxZoom=18 does not match max tile z 17`. The unverified inspector now reports both header and actual directory extrema and `headerZoomRangeConsistent=false`; never claim that the untouched archive passed PMTiles verification. User may extract a separate `--maxzoom=17` copy and compute a *new* checksum, preserving the original for provenance.

With this fixed bbox the required XYZ tile set is present for each zoom 13–17, but **no z18 tiles exist**, and source-region provenance, derivative/redistribution rights, actual native DEM sample resolution, and vertical datum remain **unverified**. Real-source production `Generation Ready` is still blocked; the present sample is usable for private, *untrusted format* comparison only. Uploading real archive bytes to public GitHub remains prohibited without explicit clearance.

## Remaining acceptance

The lead's source protocol lives at `docs/agent-handoff/MAPTERHORN-SAMPLE-PROTOCOL.md`. The regional URL or metadata text is not a release pin; the actual SHA above pins this exact *extract*, not the original upstream release. Real source datum/licensing/resolution verification, independent native Rust sample comparisons, disconnected Windows GUI/native IPC integration and native mesh/export integration are still pending. Both A2-01 (#3) and A2-02 (#4) stay open and unmerged.

## Corrected owner-supplied real regional extraction — independently authenticated

After the initial `interlaken-test.pmtiles`, the owner privately supplied **`interlaken-z17.pmtiles`** generated with explicit `--maxzoom=17`. The owner ran the official go-pmtiles verifier and reported `Completed verify in 0s` with header `minZoom=13,maxZoom=17`.

Independent byte-level comparison of both uploaded files established:
- **Corrected file:** exactly **3,127,295 bytes**, SHA-256 **`75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0`**, matching the owner's PowerShell checksum.
- **Original file:** same length, SHA-256 **`40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce`**.
- **Exactly one byte differs:** absolute **file offset 101** (PMTiles v3 header `max_zoom`), **`0x12` / 18 → `0x11` / 17**. Every other byte — including the directory, metadata, indexed tile payloads and all 40 WebP files — is identical. This eliminates the original header/directory mismatch **without replacing or changing any source elevations**.
- Independently decoded gzip root, attribution-only metadata, 40 indexed WebP images and all 512×512 RGB payloads; zoom counts remain **z13:1, z14:2, z15:4, z16:9, z17:24**. Header and actual index extrema now agree. Original first z13 center pixel RGB `(130,51,192)` evaluates to `563.75` via the documented Terrarium equation; observed formula-value range across image pixels `562.5`–`1611.0` (metres *on the claimed encoding scale only*, not datum-verified heights).
- **Do not commit either privately supplied PMTiles binary or redistribute it.** A separate private JSON evidence worksheet may contain hashes, ranges and metadata only.

**Resolved:** the PMTiles v3 max-zoom verification failure for this extract; availability of a genuinely SHA-pinned, 512px, independently decoded regional fixture (private). **Not resolved:** metadata still contains attribution only and no explicit Terrarium encoding; registered/native importer remains fail-closed, even though the format-only inspector can read this archive. Real source release identity, upstream region/source-specific derivative/redistribution rights, native DEM resolution and vertical datum are not independently proven. `productionGenerationReady` remains false; no live Windows native/Rust CLI run was performed on the private bytes during this analysis. No G0 contract schema change.

## A1-04 latest dispatch: private native Windows smoke

Read `docs/agent-handoff/A1-04-INTEGRATION-DISPATCH-2026-10-10.md` from `main` at `ea9fc46a81c36538d374ab1c23563d4d373d1fba`. The lead runs A1-04 independently in draft PR #19. This branch remains stacked, unmerged and does not change frozen G0.

In a checkout of Agent 2 PR #16 on a trusted **Windows** machine with Rust installed and Cargo dependencies prefetched, disconnect networking and run:

```powershell
.\crates\data\scripts\inspect-private-interlaken.ps1 -ArchivePath "C:\Path\To\interlaken-z17.pmtiles"
```

The script checks **exact private SHA** `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0` and exact 3,127,295 bytes, displays `rustc --version` and `cargo --version`, then uses `cargo test --offline -p map2model-data --test private_mapterhorn -- --nocapture` and a direct `cargo run --offline -p map2model-data --bin terrain_offline -- inspect <file> 13 4275 2891`. A real file is **never** present in GitHub CI; without the private file environment variable, the opt-in test prints **SKIPPED** and does not prove private native acceptance. When enabled it checks real directory counts, five zoom-specific independent 512x512 sample goldens and z18 missing data, with all datum/rights/ready flags false. HTTP proxy refusal alone is not physical offline testing.

**Honest execution status:** Both private files were independently hashed/parsed and WebP-decoded in the private container using Python/Pillow, but this container has **no Rust/Cargo toolchain** and no DNS access to install one; the actual private file has **not** been executed through compiled native Rust here or on Windows. Windows/Ubuntu public CI validates synthetic binary tests only. The operator-run private Windows test is the remaining evidence requirement.

`crates/data/ENCODING-ATTESTATION-PROPOSAL.md` describes a signed/versioned, exact-source-SHA-bound encoding-only policy for lead review, **not** an implemented production relaxation. Real vertical datum, independently substantiated regional derivative rights/native DEM resolution and upstream release remain open gates; no production Generation Ready or G0 schema change.


## Post-acceptance ownership and source provenance (2026-10-11)

Lead has accepted the **owner-attested, physically disconnected Windows native format-only test** on the corrected real Interlaken archive (see PR #16 discussion). **Do not request another standalone geodata inspector run from the owner.** The next *application* test is the integrator's full Windows synthetic-grid→mesh→real-3MF preview, not a terrain-format CLI.

Current independently researched source provenance and deliberately unapproved production gates: [`INTERLAKEN-PROVENANCE-REVIEW-2026-10-11.md`](INTERLAKEN-PROVENANCE-REVIEW-2026-10-11.md). A more concrete source-encoding-only signed attestation and key lifecycle **proposal** (not enabled in production): [`ENCODING-ATTESTATION-PROPOSAL.md`](ENCODING-ATTESTATION-PROPOSAL.md). Original Mapterhorn publisher code explicitly writes Terrarium/WebP tiles and attribution-only PMTiles JSON; this does not prove a specific source archive's release, license, mixed vertical datum or native DEM resolution.

The `.github/workflows/a2-02-portable-windows.yml` build is retained as **manual maintainer workflow_dispatch only**; the accepted Windows format inspector is an evidence artifact, not an automatically rebuilt product deliverable. Agent 1 owns full desktop integration; no frozen V1 changes or private data uploads.
