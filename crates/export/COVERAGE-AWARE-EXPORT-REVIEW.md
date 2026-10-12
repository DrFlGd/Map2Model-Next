# Agent 3 — coverage-aware selection and export handoff

**Owner:** Agent 3 review of exporter-side requirements; Agent 2 owns PMTiles header/index/selection coverage inspection and Agent 1 owns map overlay, native IPC, UI messages and Save dialog. **Status:** reviewed proposal, not a shipped feature or a promise that bounding-box overlap guarantees a tile. No change to the six frozen G0 V1 contracts or the verified exporter.

## First visual prototype: display an honest local coverage boundary

After a user chooses an existing local PMTiles archive in the **native** file picker, Agent 2 should return a lightweight native-only, bounded summary associated with the **actual read and SHA-256 verified local file**. It may be done before the separate per-job *experimental mesh/3MF* consent, but may not authorize decoding/export. Suggested G2-only DTO:

~~~text
LocalPmtilesCoveragePreviewV1 {
  kind: "LocalPmtilesCoveragePreviewV1",
  sourceArchiveSha256: <64-char lowercase SHA of actual selected bytes>,
  headerBboxWgs84: [west,south,east,north],  // native header envelope, not confirmed tile coverage
  indexFootprintBboxWgs84: [west,south,east,north] | null,
  minZoom: u8, maxZoom: u8, inspectedZoom: u8 | null,
  coverageBasis: "header-envelope-only" | "indexed-tiles",
  requestedSelectionId: string | null,
  requestedSelectionBboxWgs84: [west,south,east,north] | null,
  overlapStatus: "inside" | "partial" | "outside" | "unknown",
  indexCompleteForClosedSelection: boolean | null,
  requiredIndexedTiles: bounded u32 | null,
  presentIndexedTiles: bounded u32 | null,
  missingIndexedTiles: bounded u32 | null,
  productionGenerationReady: false,
  experimentalOnly: true
}
~~~

These are **proposed experimental UI inspection fields**, *not* a new G0 contract, not Agent 2's current `ExperimentalLocalTerrainPreviewV1` and not an assumption that Agent 2 has implemented coverage reporting. Agent 2 may choose different exact field names after review. Never expose arbitrary local paths to the renderer; the native picker owns source identity. No external network fetch or user data upload.

### Semantics Lead should use on the map

1. **Visual map shape:** render the PMTiles **header bbox** as a clearly labeled *coarse advertised envelope* if that is all the importer knows. Do not label its edges a precise coverage perimeter when the index is sparse or irregular. If Agent 2 computes an indexed-tile footprint or tiles-at-zoom rectangles, use that as a separately labeled finer coverage shape; a single bbox cannot represent internal gaps.
2. **Selection classification:** `outside` = no positive-area overlap with known coverage; `partial` = intersection, but some of the selected area/tile footprint is missing; `inside` = *all closed-seam indexed tiles required at the selected zoom are present* (only when using indexed-tile evidence). `unknown` = invalid, unverified, absent/mismatched zoom, header-only or stale coverage evidence. Touching only at a boundary is not positive-area coverage; closed XYZ seam neighbors are still required for actually decoding an edge.
3. **Header bbox overlap is advisory**, not a definitive inside result: if `coverageBasis=header-envelope-only`, show `inside envelope (not yet tile-checked)` as distinct explanatory UI text, but its export authorization status is **unknown**. A sparse PMTiles index can have an inner hole while the selection is entirely inside the advertised bounding rectangle.
4. **Source binding/staleness:** compute the classification on each drawn rectangle change with the selected archive SHA, the current *validated* `SelectionV1.selectionId`, WGS84 bbox and zoom; invalidate when source, selection, zoom or consent job changes. Full-index coverage from zoom 12 never proves zoom 17. Avoid inferring real datum, source rights or native DEM resolution.
5. **UI policy (Lead):** `inside` may enable requesting an explicit private experimental job consent and real decode; `partial/outside/unknown` disable export and explain how to redraw or select another source. No silent auto-clipping, filling blank regions, falling back to a different zoom or substituting unknown heights. A future explicitly user-approved subset/clip creates a NEW selection ID and NEW per-job consent.

## Export validation / warning review (Agent 3)

**No weakening and no new condition on the already verified G0 3MF writer is required.** Existing `validate_build` checks that metric clip lies within the decoded grid, its nodes are aligned and sampled, and `build_mesh` fails with `INVALID_DIMENSIONS` for out-of-grid clips and `DATA_MISSING` for an absent height under the clip. The verified `prepare_export` checks grid/request/source association and validates real geometry; a source's `coverageStatus=envelope-only` is recorded honestly in provenance, **not** treated as a positive indexed-tile claim. This remains appropriate.

**G2-only admission rule (before new experimental mesh/export adapter):** require a fresh **Agent 2 native index-coverage result with `overlapStatus=inside`, `coverageBasis=indexed-tiles`, `indexCompleteForClosedSelection=true`, matching exact source SHA, validated selection hash/ID + bbox, and same zoom as the candidate preview and native per-job consent.** Reject absent/partial/outside/unknown and any stale/header-only claim without creating or staging a 3MF. An index `inside` still does *not* guarantee WebP decode, alpha/no-data validity, datum, source terms or native accuracy: Agent 2's actual candidate decoder must succeed and its candidateHeightsSha256 and unknown/false flags must be validated by the separate Agent 3 adapter.

The UI's green inside badge is **not a trusted export input**. Agent 3 accepts *native-authenticated coverage evidence* or requests Lead to re-call Agent 2's independent index/closed-seam/real-decode checks before export. Do not put a `coverageStatus=inside` boolean into frozen `TerrainSourceManifestV1` or forge verified `ElevationGridV1`. Reject known prohibitive source terms even if the user accepts the private-use disclaimer.

| Condition | Native/export behavior | Safe user-facing explanation |
| --- | --- | --- |
| Inside indexed tiles + valid complete candidate decode + fresh per-job consent | May proceed to experimental mesh QA/3MF; `experimentalOnly=true`, no verified rights/datum | "Inside the local file's indexed tile coverage. Terrain values are experimental; licensing, vertical datum and resolution are unverified." |
| Partial indexed overlap / one or more missing required tiles | **No export**; `DATA_MISSING` | "This rectangle crosses an area with no locally indexed tiles. Redraw entirely inside the coverage boundary." |
| Outside known coverage envelope / no intersecting indexed tiles | **No export**; `INVALID_SELECTION` or `DATA_MISSING` as native policy decides | "This rectangle is outside your selected local PMTiles coverage. Choose another area or file." |
| Header envelope only, missing zoom, stale inspection, index cannot be authenticated | **No export**; `INVALID_SOURCE`/`DATA_MISSING` | "The visible outline is an approximate PMTiles bounding box; tile coverage has not been confirmed." |
| Inside index but corrupted WebP, missing alpha sample, source mutated or SHA changed | **No export**; `DATA_MISSING`/`HASH_MISMATCH`/`INVALID_SOURCE` | "Tiles may be indexed but the selected elevations cannot be read or validated from this file." |
| Inside fully indexed and printable but unknown datum/license/native resolution | Only **separately consented experimental local** mode; never verified | "EXPERIMENTAL — PRIVATE LOCAL USE — UNVERIFIED. Source permissions, vertical datum and native accuracy not independently confirmed." |

In the exported **experimental** 3MF's Core XML text and embedded JSON provenance, record at least `sourceArchiveSha256`, `selectionId`, requested `bboxWgs84`, `zoom`, `candidateHeightsSha256`, consent/job identity, **coverage inspection basis/selected zoom and status** (without asserting verified dataset coverage beyond that index), source attribution/warnings and the existing `experimentalOnly=true`, `redistributionApproved=false`, `verticalDatumStatus=unknown`, `nativeResolutionVerified=false`, `sourceLicenseVerified=false`. Preserve exact original policy disclaimer; no public local path or filename. A separate UI warning alone cannot replace in-file experimental provenance. No change to verified 3MF Core parts or golden hashes.

### Required coverage-focused tests before Lead's new preview

- Bbox *inside* but hole/missing tile in the PMTiles directory: map reports partial/not certified; export returns `DATA_MISSING`. Same for missing seam neighbor at an exact tile edge.
- Partial overlap across west/east/north/south edges: status partial; no export or staged file. Fully outside and touching-only cases: outside; no export.
- Header-only bbox with absent tile index detail: status unknown for export, even when rectangle geometrically falls inside the header bbox. Different zoom, stale archive SHA, altered selection ID, replaced local file or re-open must invalidate.
- A complete 512px WebP original synthetic fixture at indexed zoom 12: inside status; native decode and separate G2 candidate mesh/3MF succeeds only after exact-job consent; independent ZIP/OPC parser confirms source SHA, bbox/zoom, full experimental warning and original provenance. Negative alpha/no-data must still fail after an index-inside result.
- Engine-only regression on *original synthetic in-memory verified* grids should continue to refuse an out-of-grid clip and absent clipped elevation sample; these do not prove Agent 2 actual archive coverage.
- Cancel/worker abort/destination denial should leave no successful local artifact; no `Generation Ready` or rights/datum claim. Real data/outputs stay private and out of CI/release artifacts.

**Ownership:** Agent 2 computes actual, SHA-pinned PMTiles header/index footprint and status; Lead displays overlays, native file/zoom/selection identity, messages, consent and workflow disable/enable; Agent 3 makes the separate experimental exporter fail closed if actual authenticated coverage+decoded candidate is absent, preserves metadata/geometry/atomic output and tests it on original synthetic PMTiles once combined. Existing PR #15 remains draft; no independent merge.
