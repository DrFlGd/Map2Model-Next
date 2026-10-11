# Owner decision — Experimental Local Terrain (ELT), 2026-10-11

**Status:** APPROVED PRODUCT DIRECTION by the owner, **not** a finding that all Mapterhorn dataset rights, derivative-export permissions, or height datums are verified. Implementation is a **draft Agent 2 native-only preview** for Lead integration review. Until that integration lands, it is **not a desktop feature** and must not be described as an accepted Windows build.

## Intent
Enable an end user to choose a **privately acquired local PMTiles file** and use its documented Terrarium-encoded 512×512 WebP elevation pixels to prepare a terrain preview for **personal, local, non-distributed experimentation** (eventual local 3MF export). Do not require full external per-contributor attestation before *trying* to generate a candidate raster from the user's own file. Neither automatic data downloads nor redistributing raw DEM tiles, project packs or derived models is part of this authorization.

**Personal-use status does not waive external terms.** Users must obtain the source lawfully and follow applicable access, attribution, caching and derivative-use terms. We do **not** claim a blanket private-use exception or endorse prohibited use. Provide and retain Mapterhorn/underlying provider credit in UI and any local 3MF's provenance/notice. Prohibit publishing, selling, sharing or distributing the preview/derived outputs through the application's experimental flow until applicable rights are reviewed.

## Native separation from verified mode

- **Verified/production mode (unchanged):** `import_local_pmtiles`, registered immutable `TerrainSourceManifestV1`, `decode_selection_grid`, known verified datum, source rights and native resolution requirements, validated `ElevationGridV1`, existing Generation Ready checks. No bypass or reinterpretation of these claims.
- **Experimental-only preview mode (new draft):** `terrain_offline experimental-local ...` requires **an explicit uppercase opt-in acknowledgment** and **SHA-256 of the exact local PMTiles bytes**, plus a frozen validated `SelectionV1` and requested zoom. It uses native bounded/offline PMTiles v3 index parsing and WebP Terrarium *candidate* decoding with closed geographic coverage and alpha/no-data fail-closed checks. It tolerates the **attribution-only** metadata actually emitted by Mapterhorn's source-code pipeline; a conflicting explicit non-Terrarium encoding rejects.
- The preview output is **`ExperimentalLocalTerrainPreviewV1`**, NOT `ElevationGridV1` and NOT `TerrainSourceManifestV1`. It records `verticalDatumStatus="unknown"`, `sourceLicenseVerified=false`, `nativeResolutionVerified=false`, `encodingAuthenticated=false`, `productionGenerationReady=false`, the true archive SHA, bbox/selection ID, geographic anchors, real bounded pixel sample array and attribution. An unverified *numeric* candidate Terrarium height is not a verified geodetic elevation datum.
- No feature switch allows preview output to be registered as a production source or spoof an internal authorized token. A future optional Lead-owned mesh/3MF bridge must enforce the separate **experimental/private-only** provenance watermark on the job, exported 3MF package, UI and saved project. It must not strip this status when converting candidate samples to an ephemeral mesh input.
- Selection must fit actual header envelope AND every referenced closed-boundary tile must exist in the index. Alpha no-data, missing tiles, insufficient grid budget, bad SHA, malformed WebP, inconsistent zoom, missing opt-in, untrusted symlink and file mutations all fail before create-new output.
- Never auto-download outside explicitly approved provider policies; input and output stay on disk; no embedded private Interlaken bytes in public repo/Actions. Do not silently write public-facing fixture blobs or upload projects/3MF.

## Minimum user-facing acknowledgment before each experimental job

> Experimental local terrain. I acquired this dataset under the applicable terms. The vertical reference, contributing source licenses, native accuracy and absolute heights are not independently verified by Map2Model Next. This is for my private local evaluation; I will keep provider attribution and will not publish or redistribute generated files without separate rights review. I understand this is not Generation Ready certified output.

Require an explicit user checkbox / opt-in in the integrated desktop UI, not a prechecked default; bind it to the specific local SHA and selection job. The native CLI uses a long exact opt-in token only for developer testing; it does NOT replace a trusted desktop consent UI.

## Milestones and ownership

1. **Agent 2 draft scope:** implement bounded candidate Terrarium sampling, honest preview JSON and synthetic negative tests. Verify on Windows/Ubuntu CI without touching private Mapterhorn. Publish draft PR #16 comments with SHA/test evidence and Lead handoff; no self-merge.
2. **Agent 1 Lead + Agent 3 integration review:** connect explicit local preview path to the already functioning native mesh/real 3MF writer, create a clearly marked personal experimental provenance/notice, secure local file picker + Save dialog, job cancellation/error lifecycle and UI warnings. The frozen six V1 contracts must not be rewritten to fake known datum; define the separate experimental job envelope or bridge explicitly.
3. **Product-owner next manual test:** compiled **integrated Windows preview**, not another standalone inspector. Verify personal file import, map selection, actual preview mesh/3MF in PrusaSlicer and Bambu Studio, visually inspect warnings/attribution, and confirm offline behavior. Previous 40-tile real-format-only Windows acceptance does not establish a real-source-to-3MF pass.
4. **Future verified release mode:** source-contributor/release lineage, licenses, native resolution, vertical datum harmonization and signed encoding evidence remain independent acceptance criteria.

Source background: Mapterhorn's publisher documents Terrarium/WebP and creates attribution-only PMTiles; the local archive's original SHA is `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0`. Original format inspection passed physically offline on the owner's Windows host. Those facts support an experimental preview but do not establish universal permission to redistribute any raw or derived real terrain.
