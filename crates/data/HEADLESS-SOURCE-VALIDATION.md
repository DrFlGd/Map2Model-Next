# A2-02 — Native offline terrain CLI handoff (synthetic-first)

**Status:** Implemented in stacked draft PR #16. This is a **developer-only native tool**, not Electron IPC or a user-ready source acquisition service. The frozen G0 contracts are untouched. No network access or automatic downloads exist in the binary.

## Commands

### 1. Inspect an externally acquired local archive without authorizing it

```sh
cargo run --offline -p map2model-data --bin terrain_offline -- inspect ./interlaken-private.pmtiles

# Where z/x/y are independently verified actual indexed tiles:
cargo run --offline -p map2model-data --bin terrain_offline -- inspect ./interlaken-private.pmtiles 13 4275 2891
```

The second command selects one actual indexed tile and decompresses/decodes its **512×512 WebP**, returning the signed Terrarium north-west, center and south-east pixel heights, masked pixel count and encoded tile length. This inspection still requires the current bounded PMTiles v3 supported subset (≤128 MiB, ≤200,000 indexed tiles, WebP, directory modes none/gzip and encoding declared `terrarium`). An authentic extract outside the supported subset should be reported as incompatible, not silently substituted or interpreted.

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

## Real Interlaken acquisition status

The lead's protocol: `docs/agent-handoff/MAPTERHORN-SAMPLE-PROTOCOL.md` specifies the candidate `bbox=7.88,46.68,7.89,46.69`, regional source URL and provenance worksheet. This candidate and any z/x/y sample are **not** release pinned by a URL alone. No verified local extract is available in this build environment; official Mapterhorn binary endpoints were inaccessible from the sandbox when checked. **Do not fabricate a local archive hash, datum ID, license or a production-readiness statement.** Test genuine source only from a lawful, privately acquired, pinned file; do not publish data bytes without clearance.

Remaining acceptance: real source metadata/rights/datum/native-resolution verification, golden pixel comparison to an independently acquired extract, disabled-network Windows native IPC end-to-end, and native mesh/export integration beyond the tested headless synthetic demonstration. Both A2-01 (#3) and A2-02 (#4) stay open and unmerged.
