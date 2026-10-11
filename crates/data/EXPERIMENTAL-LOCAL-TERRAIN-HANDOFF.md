# A2-02 experimental local terrain — native preview handoff

**Decision:** Product owner chose the *Experimental Local Terrain* personal workflow on 2026-10-11. See [policy](EXPERIMENTAL-LOCAL-TERRAIN-POLICY.md). **Status:** native geodata candidate-preview path implemented and tested, desktop 3MF flow **pending Lead/A3 integration**. This is not a source licensing opinion or blanket waiver of external terms. No private user's tiles or generated raster are included in CI/repo.

## Exact new native API

```rust
map2model_data::experimental::decode_experimental_local_pmtiles(
    file: &Path,
    expected_sha256: &str,
    selection: &SelectionV1,
    zoom: u8,
    acknowledgement: &str,
) -> Result<serde_json::Value, String>
```

The native developer command is:

```text
terrain_offline experimental-local <local.pmtiles> <expected-sha256> <selection.json> <zoom> I-ACCEPT-UNVERIFIED-TERRAIN-FOR-PRIVATE-LOCAL-EXPERIMENTATION <new-output-preview.json>
```

This command must only be invoked from a *native-local, owner-consented and SHA-pinned* file picker flow. It outputs a **new** `ExperimentalLocalTerrainPreviewV1` JSON document with real, bounded, north-up candidate Terrarium samples. Command stdout returns only `previewPath`, source SHA, sample-array SHA, selection ID, dimensions and unknown/false source-readiness flags. **It does not save a mesh or a 3MF.**

### Preview DTO — distinct from all frozen G0 contracts

| Field | Meaning |
| --- | --- |
| `kind` | `ExperimentalLocalTerrainPreviewV1`, not `ElevationGridV1` or `TerrainSourceManifestV1` |
| `experimentalOnly`, `localPersonalUseOnly`, `ownerAcknowledgedUnverified` | Always `true` after exact consent |
| `sourceArchiveSha256`, `sourceArchiveBytes` | Exact verified **local file bytes** |
| `sourceAttributionFromArchive` | Literal metadata text (untrusted for HTML; **escape in renderer**) or null |
| `sourceMetadataDeclaresTerrarium` | Genuine metadata flag, false for attribution-only Mapterhorn archives |
| `encodingAuthenticated` | Always `false`, even if numerical Terrarium WebP decoding works |
| `selectionId`, `bboxWgs84`, `zoom`, `anchorWgs84` | Frozen validated selection identity, explicit geographic coordinates |
| `width`, `height`, `dxMeters`, `dyMeters` | Bounded, physically scaled sampled grid |
| `storageConvention` | `north-up-f64-row-major` |
| `candidateTerrariumHeightsMeters` | Real 512×512 WebP pixel-derived **candidate** heights, not certified geodetic elevations |
| `candidateHeightsSha256` | Domain-separated SHA over local SHA, selection ID and row-major f64 samples; **NOT** a `gridSha256` |
| `verticalDatumStatus` | ALWAYS **`unknown`**, no forged `verticalDatumId` |
| `sourceLicenseVerified`, `nativeResolutionVerified`, `redistributionApproved`, `productionGenerationReady` | ALWAYS `false` |
| `warnings`, `networkRequests` | Local-only UI disclaimers and zero network requests |

`selectionId` uses frozen G0 selection validation. No change to any frozen schema or `canonical::grid_digest`, and no suggestion that the above DTO can be fed directly to the *trusted* grid/mesh contract verifier. The **Lead owns** the *explicit experimental-only bridge* to in-memory mesh input and distinct local-only 3MF provenance; avoid producing a fake certified source manifest/vertical CRS just to satisfy that mesh's verified input requirement.

### Required Lead/Agent 3 experimental mesh→3MF acceptance

1. User picks an existing **local** file; it is not uploaded or implicitly fetched. UI shows source accreditation + warning and requires an **unchecked-by-default, per-job** acceptance checkbox. Bind accepted acknowledgement to source SHA and selection/job ID; never persist global blanket consent.
2. Native process validates exact SHA-256 and file regularity, reads ≤128 MiB, parses genuine indexed PMTiles v3 512px WebP, validates entire CLOSED geographic selection coverage (both seam neighbors), rejects all missing/alpha/invalid data, bounded 100k preview samples and max 64 cached tiles. No disguised fallback to lower resolution or fake missing heights.
3. Build a mesh only from an **explicitly branded experimental input**, never by forging `verticalDatumStatus=known`. User-facing 3MF/package manifest must store `experimentalOnly=true`, `verticalDatumStatus=unknown`, candidate source SHA, selected bbox/zoom, source attribution, and “local personal evaluation—not certified elevations; redistribution unapproved” notices. Do not label ordinary `Generation Ready` true.
4. Keep UI/worker job lifecycle, cancellability, safe local file picker, native permission checks, file export Save dialog and error codes. No network access required for already acquired PMTiles.
5. Keep primary verified importer path `import_local_pmtiles` and `decode_selection_grid` unchanged and fail-closed without a trusted encoding/datum declaration. The experimental path is separate **by command and output type**.
6. Test resulting **real-source experimental 3MF** locally in actual PrusaSlicer and Bambu Studio only in the **next integrated Windows preview**, avoiding another stand-alone inspector. A successful synthetic decoder test or prior real-file format test is not a full application/export pass.

### Regression evidence

At exact branch SHA `e505d119cb9568f66ea62aab1769d62c2ae1ce79`, [Ubuntu and Windows full-workspace CI passed](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38157149630) and the [original synthetic handoff CI passed](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38157149632). In `crates/data/tests/experimental_local.rs`, **3/3 real native subprocess tests** passed on each operating system:

- Generated originally owned (CC0) **complete** 4-tile 512px PMTiles source → preview 6×8 actual sampled values with changing heights, exact SHA and consent; source datum/license/ready statuses still unknown/false, and `ElevationGridV1` deserialization rejects preview DTO; output file cannot be overwritten.
- Generated **missing southeast** and **alpha no-data southeast** PMTiles → `DATA_MISSING`, zero candidate output; wrong SHA `HASH_MISMATCH`, absent consent `EXPERIMENTAL_CONSENT_REQUIRED`.
- Same generated original WebPs with **attribution-only** metadata (real archive shape) → success **with metadata-declared Terrarium false and encoding-authenticated false**; an explicitly conflicting encoding string → `INVALID_SOURCE`.

**Not yet exercised in CI:** direct experimental sampling of the owner's private Interlaken binary; on-owner physical Windows preview from the **new integrated 3MF app**. The prior owner-attested offline real Interlaken *format* inspector PASS is separately recorded, not incorrectly claimed to cover this new experimental mode.

External data-usage restrictions remain in force and shall be explained; private non-distribution alone does not legally waive dataset-specific terms.
