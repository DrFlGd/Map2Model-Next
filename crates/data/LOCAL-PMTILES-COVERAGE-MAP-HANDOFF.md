# Agent 2 → Lead: offline PMTiles visual coverage overlay (2026-10-11)

**Scope:** Latest owner priority. After the user selects a **locally held PMTiles archive**, draw at least the declared PMTiles WGS84 coverage boundary on the MapLibre map and classify the user's drawn rectangle as `inside`, `partial` or `outside` *at their desired source zoom*. Agent 2 owns native metadata/index evaluation; Lead owns OS file-picker, map geometry rendering, warnings and UI lifecycle. Agent 3 reviews coverage-aware export warnings. The new API is **experimental-only**, and does not modify the frozen G0 six wire contracts.

## Delivered native API (private geodata branch PR #16)

```rust
use map2model_data::coverage::{
    inspect_local_pmtiles_coverage,
    CoverageSelectionQuery
};

// On native OS-selected file (no renderer-controlled arbitrary filesystem).
let overlay = inspect_local_pmtiles_coverage(&local_file_path, None)?;

// Re-run after user changes drawn rectangle; binds to file-picker source SHA:
let query = CoverageSelectionQuery {
    expected_source_sha256: overlay["sourceArchiveSha256"].as_str().unwrap().to_string(),
    bbox_wgs84: [west, south, east, north], // raw drawn rect, WGS84
    zoom: selected_source_zoom,
};
let assessed = inspect_local_pmtiles_coverage(&local_file_path, Some(&query))?;
```

No `SelectionV1` is required for live rectangle edits. **Before preview/export**, Lead must still create/validate the frozen hashed `SelectionV1` and obtain separate per-job consent bound to source SHA and operation.

Native CLI examples (developers/integrator only, **not another requested owner inspector test**):

```text
terrain_offline coverage <user-selected-local.pmtiles>

terrain_offline coverage <user-selected-local.pmtiles> <exact-original-source-sha256> \
   <west> <south> <east> <north> <requested-zoom>
```

These commands output **`LocalPMTilesCoveragePreviewV1` JSON only**, not elevation samples or data files. No consent needed merely to display selected source coverage. Rectangular input is checked for finite coordinates, WGS84 limits and ordered corners; malformed bbox and invalid zoom are rejected with `INVALID_SELECTION`. Second invocation rejects changed source bytes with `HASH_MISMATCH`; native OS input must remain the user-picked path, with no renderer-controlled path injection or network fetch.

## Stable fields and meaning

| JSON | Meaning |
| --- | --- |
| `kind` | `LocalPMTilesCoveragePreviewV1` |
| `sourceArchiveSha256`, `sourceArchiveBytes` | Exact locally hashed PMTiles bytes and safe input length |
| `declaredCoverageBboxWgs84` | **header-declared** `[west,south,east,north]` rectangle in EPSG:4326 |
| `boundarySource` | String warns the header bbox **can overstate actual tile coverage** |
| `coverageGeoJson` | MapLibre-ready Polygon `Feature`; lon/lat points in ring order west/south→east/south→east/north→west/north→west/south |
| `headerMinZoom`, `headerMaxZoom` | Declared archive zoom envelope; **not** actual source resolutions |
| `indexedTilesByZoom`, `indexedTileCount` | Counts derived from the parsed **actual directory tile IDs**, not inferred from header |
| `selection` | `null` until a drawn bbox is supplied; then the **index-only** coverage verdict |
| `sourceLicenseVerified`, `nativeResolutionVerified`, `productionGenerationReady` | Always `false` in the coverage preview |
| `verticalDatumStatus`, `networkRequests` | Always `unknown`, `0` |

`selection` object with a drawn bbox:

- `bboxWgs84`, `requestedZoom`, `zoomAvailable`
- `status`: exactly **`inside` / `partial` / `outside`**
- `insideDeclaredBbox`, `intersectsDeclaredBbox`, optional `intersectionWithDeclaredBboxWgs84`
- `requiredClosedBoundaryTiles`, `indexedRequiredTiles`, `missingRequiredTiles`, `allRequiredTilesIndexed`
- `classificationScope`: explicitly **index only**, not decoded alpha/no-data/height availability

**Status semantics:**

1. **Inside:** selection lies *wholly* within declared bbox **AND** all closed-boundary XYZ tiles requested by rectangle are present in the real index at the requested zoom. This does **not** guarantee every RGB pixel is valid or authorize generation.
2. **Partial:** selection has genuine geometric overlap with declared bbox and **some but not all** required XYZ tiles are indexed, or required tiles are indexed but part of rectangle is outside declared bbox. Display a non-green warning ("Some requested area is not covered"), with `missingRequiredTiles` and clip intersection rectangle if useful.
3. **Outside:** zero actual indexed tiles intersect selection at zoom, or no positive-area header-bbox overlap. If the zoom is missing, `zoomAvailable=false` and status is outside; UI should explain unsupported zoom rather than "no dataset".
4. **Over-budget:** return a typed `OUT_OF_MEMORY` error if the rectangle spans more than 4096 indexed-lookup candidates; do NOT fabricate a status. UI should ask user to zoom in/reduce rectangle.
5. A zero-area boundary touch is treated as **outside positive-area intersection** for map-only classification (except when the rectangle remains otherwise inside declared bbox); **actual decode** still has stricter closed-rectangle tile ownership on all edges.

**Important data distinction:** The GeoJSON boundary is **not** computed from the union of tile footprints; it is explicitly the bounded, validated PMTiles header envelope. A source archive can have holes and different tile coverage at each zoom. The native checker reads all directory entries and validates WebP containers but **does not decode/validate every RGBA alpha pixel** to produce status. Therefore `status=inside` means **inside declared bbox with every required XYZ tile indexed**, not "every sample available", "rights approved", or "Generation Ready". Lead must call the normal explicit-consent experimental decoder before moving to an experimental mesh export; decoding still fails `DATA_MISSING` on selected alpha/no-data samples.

## Visual flow owned by Lead

1. OS-provided local file picker → `coverage <native-selected-file>` → verify returned SHA, draw `coverageGeoJson` as a distinct source boundary (e.g., outlined/fill-transparent polygon; mark **Declared coverage — may contain missing tiles**). Fit/map zoom to `declaredCoverageBboxWgs84` as appropriate.
2. User draws/edits rectangle in MapLibre → debounced `coverage <same-native-selected-file> <original-sha> <drawn-bbox> <zoom>` through **allowlisted native IPC**. Renderer supplies only numeric selection rectangle/zoom, not an arbitrary file path or CLI operation.
3. Show **Inside indexed coverage**, **Partially covered**, **Outside**, or **Selection too large / source changed**; missing counts, zoom mismatch and map boundary retained. Never color/label inside as a guarantee of usable elevation samples.
4. Do not permit experimental preview/export based *solely* on the index-only inside result. Explicit local-use acknowledgment, checked SHA, frozen `SelectionV1`, decoded no-data validation, separate experimental 3MF notice remain mandatory. Outside and partial must not be silently clipped or auto-filled; offer user an edited selection before starting the job.
5. No real Mapterhorn files or real-derived models in the application release/CI. No download or server acquisition. This is **not** the next user test: Lead bundles map UI and the integrated real-source experimental 3MF preview in the next Windows prototype.

## Synthetic tests + reference

`crates/data/tests/local_coverage.rs` uses only the preexisting original generated four-tile z12 512×512 PMTiles fixtures. Tests include file-selected GeoJSON polygon ring / exact source SHA, all-four-tile **inside**, partial header crossing with missing indexed tile, **outside** envelope, header-inside yet **partial** on missing southeast tile, header-intersecting **outside** if the only needed tile is absent, accepted **index-only inside** on masked RGBA tile (warning still no-data pending), unsupported zoom, invalid bbox/NaN/zoom, wrong SHA, oversized tile query and malformed archive. Physical private Interlaken remains owner-held and never uploaded.

**Agent 3 question:** Should the 3MF validator/warning envelope report the original user selection `coverageStatus`, `indexedRequiredTiles` and `missingRequiredTiles` as experimental context? If yes, transport them as **experimental-only metadata, never trusted data rights/datum**. Never permit an `inside` map coverage assessment to override runtime decode `DATA_MISSING`; partial/outside preview/export should block or demand a newly valid selection, not clip silently.

**Expected next agent steps:** Agent 2 verifies current branch SHA and Windows/Ubuntu CI, posts PR #16 + issue #4 evidence; Lead integrates map overlay and exported-3MF flow, Agent 3 comments on exporter only. No self-merges or G0 rewrites.
