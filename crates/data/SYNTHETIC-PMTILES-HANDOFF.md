# A2-02 original synthetic PMTiles → grid → mesh handoff (post v0.0.2)

**Status:** Agent 2's synthetic-only integration fixture / strict test evidence. **Not** a real geographic DEM, no Mapterhorn download, and **no real-source Generation Ready**. Do not ask the owner to repeat the already accepted standalone inspector or v0.0.2 basic dual-slicer smoke; Lead owns the next packaged Windows preview.

## Deterministic, original test content

Generator: `crates/data/src/bin/terrain_synthetic_fixture.rs`, built as `terrain_synthetic_fixture` by the existing `map2model-data` package. It creates **real PMTiles v3** archive bytes at zoom **12** using the actual original **512×512 lossless WebP** encoder and no third-party terrain input. It writes separate content-addressed, frozen-compatible `SelectionV1`, SHA-pinned source declarations, and a small report. The data is created by a simple invented mathematical function, `height_meters = -96 + globalPixelX/8 + globalPixelY/16`. This relative height convention is **SYNTHETIC-RELATIVE-METERS only**, not Earth's vertical datum. Generated mathematical test pixels are contributed for this repository under CC0-1.0; source declarations do **not** authorize any other data.

The complete indexed tile set is **z12** x=1091/1092 and y=1554/1555; the selection bbox is **`[-84.024,39.7067,-84.023,39.7077]`**. It spans **both** actual Web Mercator XYZ seams at x1092 and y1555, so it tests real four-tile grid-node ownership and conservative 4-edge source coverage. The exact rectangular selection is fixed and is **not** the user's arbitrary map area. `targetGroundSampleMeters=18`, `noDataPolicy=fail`, `maxSamples=10000` ensure bounded output.

Three **separately SHA-pinned** variants:

| Generated asset | Contents | Expected native decode |
| --- | --- | --- |
| `complete.pmtiles` | Four genuine RGB 512px WebP tiles, present around two-axis seam | **Success:** genuine `ElevationGridV1` with repeatable content SHA, synthetic test datum |
| `missing-southeast.pmtiles` | Southeast Z/X/Y tile **absent** from real index | **DATA_MISSING**, no grid output |
| `masked-southeast.pmtiles` | Four indexed tiles, southeast image **fully RGBA alpha=0** | **DATA_MISSING**, no filling masked heights |

Each has a corresponding `<variant>.source.json` with `expectedArchiveSha256`, and all use `selection.json`. `fixture-report.json` contains exact SHA values, file names, selected bounds and explicit false real-production-readiness. The generator refuses to overwrite an existing directory.

## Simple CLI-to-Lead handoff (developer/CI, not owner instruction)

Run this from the stacked Agent 2 PR #16 checkout, with compiler dependencies cached (or use the safe synthetic-only GitHub artifact in the companion Actions workflow):

```sh
cargo run --locked -p map2model-data --bin terrain_synthetic_fixture -- ./new-original-synthetic
cargo run --locked -p map2model-data --bin terrain_offline -- inspect ./new-original-synthetic/complete.pmtiles 12 1091 1554
cargo run --locked -p map2model-data --bin terrain_offline -- decode \
  ./new-original-synthetic/complete.pmtiles ./private-artificial-registry \
  ./new-original-synthetic/complete.source.json \
  ./new-original-synthetic/selection.json \
  12 ./new-original-synthetic/decoded-grid.json
```

The `decode` tool writes the **actual frozen `ElevationGridV1` JSON** only if the destination does not exist. It prints `sourceArchiveSha256`, `manifestSha256`, `gridSha256`, `selectionId`, dimensions, `networkRequests=0`, and **`productionGenerationReady=false`**. The JSON can be passed to Agent 3's `map2model-mesh::build_mesh` API with a matching `TerrainBuildRequestV1` and eventually to the native 3MF writer under Lead-controlled job lifecycle and export-path authorization. The V1 manifest is produced by the existing A2-01 registry and semantic validator using **declared synthetic** datum and original test rights; no special real-source policy exception is added.

Run tests:

```sh
cargo test --locked -p map2model-data --test synthetic_pmtiles_integration -- --nocapture
node --test crates/data/tests/encoding_attestation.test.mjs
```

The integration tests invoke **actual native CLI subprocesses**, construct the genuine grid twice through independent registry locations and demand identical sample arrays, valid masks, grid SHA, exact west/south WGS84 anchor, east/north local projected extents, nontrivial signed heights and an actual watertight/oriented Agent-3 native mesh. They assert **typed fail-closed** responses for missing southeast tile, alpha no-data, wrong source SHA, unknown vertical datum, missing license/source-resolution, insufficient grid budget, malformed PMTiles and selection coverage outside the index; create-new output cannot overwrite earlier grid.

Test-only encoding policy: `crates/data/provenance/test-only-encoding-attestation.mjs` implements a **strict flat payload RFC8785 canonical JSON + domain-separated Ed25519** signature *reference* with published dummy test seeds (never deploy those signing keys). It binds actual synthetic archive SHA + bytes, independently snapshotted **test** publisher-evidence hash, publisher commit label, release/key-policy revisions, test curator role. It rejects tampered bytes, modified evidence, duplicate/noncanonical JSON, unknown/rights/datum fields, wrong issuer, wrong key, revoked key, stale key policy, wrong source-release scope and altered signature. **Encoding-only is not licensing/datum/native-resolution approval**, even after a valid signature. The production Rust importer has **no** attestation reader/feature flag; nothing is enabled or represented as trusted Mapterhorn proof.

### CI and integration-artifact boundary

The Agent-2-only [`.github/workflows/a2-02-synthetic-handoff.yml`](../../.github/workflows/a2-02-synthetic-handoff.yml) runs native seam grid→mesh tests, strict lint, *test-only Ed25519/JCS* negatives against the actual created **synthetic** PMTiles, CLI decode, SHA and manifest/grid summary; it publishes a short-lived **original synthetic fixture artifact** for Lead's internal integration. It **never** acquires, commits, derives or distributes the private Interlaken PMTiles file. This is a lead handoff, not a second standalone Windows inspector/owner acceptance exercise.

## Source production and user acceptance remain separate

- Real Interlaken PMTiles SHA `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0`: **format-only** accepted on physically disconnected Windows. This test fixture is **not based on those bytes**.
- [Independent Interlaken source-provenance review](INTERLAKEN-PROVENANCE-REVIEW-2026-10-11.md) identifies Mapterhorn Terrarium/WebP behavior and original candidate swisstopo source terms, but not the **exact region's contributors, datum harmonization, all dataset licenses, true native accuracy or cryptographically pinned parent release**. Production source remains `UNSUPPORTED_DATUM`/unapproved.
- Signed **test-only** encoding proof MUST remain entirely distinct from real-source rights/datum. Proposed production trust policy still needs Lead approval.
- Lead owns stateful/cancellable Electron worker, actual variable-grid 3MF export, notice packaging, authorized Save dialog and next user-launchable Windows preview. No code in this fixture bypasses that integration.

Report exact new source/grid/mesh SHA evidence and CI job links on stacked PR #16; PR #14 remains draft and importer policy remains fail-closed for real unverified sources.

## Independent exact-byte CI fixture acceptance (2026-10-11)

The [original source/decoder/mesh and Ed25519 handoff workflow run 38155328272](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38155328272) passed at code commit `ed4f0db30fc4f656878503ed2b2b847c72601e70` and published the original-only artifact **`agent2-original-512px-pmtiles-to-grid`** (artifact ID `11708307259`, GitHub artifact ZIP SHA-256 `58ee8ba0ab8ea2ba73cc7f6a033df376ab36fa2164bca0cba71496b3a06f5da1`). The archive was downloaded and independently inspected as a ZIP in a separate environment; no private Mapterhorn bytes were present.

| Expected result | SHA-256 / value |
| --- | --- |
| Complete original synthetic PMTiles (108,494 bytes) | `489ec694f29c3d4c7bc5ed3f2dced736736e5c2a5d09cc1cef1ff3a0a8ede651` |
| Missing southeast tile PMTiles (81,448 bytes) | `edcf389182529e9ee8e5b8959cdd701acc2a73f1cce87fbdce8418595d5e0aa4` |
| Masked southeast tile PMTiles (112,632 bytes) | `43cacc5ca8657e9fcab11c77a8a3ed1b5f854e7d3e7473f70c5de4d7e8bd5dba` |
| Frozen `TerrainSourceManifestV1` digest (complete) | `dcc92181e381e3d4bc54134de722ae5e50975d73113d947b953c2a019cb4140b` |
| Frozen `ElevationGridV1` canonical digest (6×8) | `d8d29dfe158c9aed3d3a4dbea29ae495cf56e35668f785e4e201cb2c4f99c2b8` |
| Agent 3 actual watertight native canonical mesh digest (96 vertices / 188 triangles) | `43c7901d2b368a895cdfab30ce07cdae2305102c38ef049a13e7752eb53c8e9f` |
| Frozen `SelectionV1` identity | `sel-39d3c9ddee03b25c249251cf32659e33` |
| Synthetic encoding-only test evidence snapshot (NOT upstream publisher doc) | `9b5db0dcb1be90bcfe73fa66bd937e4b96ce199d7bdaa5af77366d832575d951` |

Real-native test results: **2/2** new Rust integration tests PASS; **7/7** Node Ed25519/strict RFC8785-subset tests PASS; zero network requests. For the good fixture, two independent decoder processes produced the same source/manifest/grid hashes and original signed sample values, the closed/watertight mesh was built through Agent 3's real Rust library; for missing and masked southeast and invalid source declarations/selection, no grid output was published. The generated `decoded-grid.json` and `decode-summary.json` are included in the GitHub artifact for the integrator. The **actual 3MF writer and Electron job lifecycle remain Agent 1/Agent 3 integration**; a mesh test is *not* an assertion that a new variable-grid 3MF export occurred.

These SHA results belong **only to original synthetic fixture code and exact dependencies at this run**. They do not describe the private Mapterhorn sample or v0.0.2 fixed-grid Windows preview. The source generator is deterministic and requires no network, but any future generator/codec changes must be re-hashed/re-tested at their exact commit. Never take `SYNTHETIC-RELATIVE-METERS` to mean a real known datum.
