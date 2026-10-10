# ADR-004 — Cross-agent V1 contracts and deterministic reference vectors

**Status:** REVISION PROPOSED; NOT FROZEN. Agent 2 and Agent 3 requested changes on PR #11 at `b533328`. This document responds to both. `G0` remains **blocked** until both agents approve the revised head and the product owner authorizes freeze.

## Normative foundation

The six `packages/contracts/schema/v1/*.schema.json` schemas are JSON-wire contracts. TypeScript and Rust mirrors must pass the **same** positive/negative fixtures. `packages/contracts/src/validation.mjs` and `canonical.mjs` provide executable reference validation and canonical encoding. Production Rust and TypeScript consumers must enforce these invariants at ingress, not simply deserialize untrusted objects. Data and exported artifacts remain local; neither agent should copy Mini Skyline's site bundles.

**Numerics:** IEEE 754 binary64 for projected coordinates and elevations, all finite. Normalize negative zero to positive zero before hashing. No NaN, infinities, interpolated missing heights or geometry outputs marked valid without passing required checks.

### C-01 Selection / CRS

Earth, WGS84 longitude/latitude, `bbox=[west,south,east,north]`, non-wrapping, strictly positive area, closed bounds. For G1 `|latitude| <= 60°`, each metric selection side <= 2,000 m, and at most 1,000,000 samples. No automatic dateline handling in G1; crossings are rejected. `selectionId = "sel-" + first 32 hex chars of selectionDigest` binds the exact bbox and terrain profile, not display names.

**Chosen G1 transform candidate:** `WGS84-ENU-LINEARIZED-V1`, a first-order local East/North tangent approximation, **not** the unqualified EPSG:4326 CRS and **not** a geodesic-preserving global projection. Input anchor `[lambda0,phi0]` in WGS84 degrees (height 0 for the horizontal transformation). Let `a=6378137 m`, `f=1/298.257223563`, `e2=f*(2-f)`, `M=a*(1-e2)/(1-e2*sin(phi0)^2)^(3/2)`, `N=a/sqrt(1-e2*sin(phi0)^2)`, with latitudes in radians. Then `east=(lambda-lambda0)*pi/180*N*cos(phi0)`; `north=(phi-phi0)*pi/180*M`. Inverse divides by these respective constants, retaining the anchor. This approximation is restricted to ±60° and 2 km per side; a larger/higher-latitude project must use a newly versioned projection. The target horizontal side-length discrepancy budget is 0.1% (2 m over 2 km) against WGS84 geodesics.

Reference: anchor `[-84.09,39.741]`, point `[-84.089,39.742]` projects to `[85.71560839369111,111.02965952283863]` m, tolerance 1e-6 m; roundtrip degrees within 1e-12. Grid node `(column,row)`: `east=minEast+column*dx`, `north=maxNorth-row*dy`. **Nodes include their outermost sample coordinates**, so spatial width/height are `(width-1)*dx` and `(height-1)*dy`, not `width*dx`. North-up means row 0 is most northerly.

### C-02 Archive provenance, manifest and coverage

`sourceArchiveSha256` hashes the **original archive bytes**. `localAssetToken="m2m-asset:"+sourceArchiveSha256` resolves only in the native sidecar's permissioned, read-only import registry; it is not a path, URL or renderer privilege.

`manifestSha256` is distinct. It excludes its own field and uses the canonical manifest wire encoding described below. `ElevationGridV1.sourceManifestSha256` MUST equal it. Corrupt or absent data fails with `HASH_MISMATCH` or `DATA_MISSING`.

`coverageBbox` is an approximate envelope, **not readiness proof**. `coverageStatus="envelope-only"` is never Generation Ready. `"verified-tile-index"` is **only a declaration** until a native registry verifies the actual immutable index bytes and requested Z/X/Y entries. Manifest validation reports `indexDeclared` (not `coverageReady`) and never certifies selection coverage. A metadata-only forged index must never enable generation. Importer validates actual tile existence and requested source release, not downloaded tile zoom as a proxy for source accuracy.

`nominalGroundSampleMeters` is a descriptive estimate only (not accuracy guarantee); `nativeResolutionSource` explains its provenance; `uncertaintyMeters` may be null. Actual per-tile quality belongs in the verified index. `verticalDatumStatus` is `known|unknown|unsupported`; only `known` can enter Generation Ready, and every source must carry `verticalDatumId`. Never silently convert MSL/ellipsoid/unknown datums. Region-specific attribution/license remain mandatory.

### C-03 Elevation grid and hashes

G1 grid: 2..4096 columns/rows, `width*height<=1,000,000` checked *before* allocation. A row-major north-up grid contains exact `width*height` pairs. A valid cell is `validMask=true, samplesMeters=finite f64`; invalid is `false,null`, with no sentinel zero. `storageMode="inline-fixture"` is for small test grids; practical production IPC uses `"local-binary-token"` with `gridDataToken="m2m-grid:<gridDataSha256>"` plus SHA-256 of the **uncompressed immutable grid payload**. The native sidecar verifies the full bytes before use. A 1 MiB JSON IPC control envelope **never** transports millions of inline floats. No raw untrusted disk path crosses the renderer; agent engines may use verified native file/memory-mapped handles.

`gridSha256` is separate from transport-byte `gridDataSha256`. The canonical grid digest is computed over the *decoded* grid and metadata using algorithm below, **excluding `gridSha256` itself**. Producer must validate all data and compute digest before persisting; consumer must independently recompute it. Fallback to an inferred datum or padding the array is forbidden. `noDataPolicy="fail"` forbids triangles touching null samples.

### Canonical binary encodings (SHA-256, V1)

All digests SHA-256 lowercase hex over concatenated byte arrays. `str` = UTF-8 length prefixed with unsigned 32-bit **little-endian** length. `hex` = exactly 32 raw bytes decoded from 64 lowercase hex chars. `f64` = finite IEEE-754 **little-endian** 8 bytes, -0 normalized to +0. `u8/u32/u64/i64` = ordinary little-endian integer of that width. String fields encode **exact supplied UTF-8 bytes**; no locale formatting and no JSON canonicalization. No timestamps/random IDs in geometry digest; refuse non-finite values and signed overflow.

- **Selection:** ASCII `M2M-SELECTION-1\0` + bbox 4×f64 (W,S,E,N) + sampleMeters f64 + noDataPolicy str + maxSamples u32. Reference SHA `9c13f5eaa44b01562cc6c26796669d44e25bd9ed593420861ef353065c9ae274`.
- **Manifest:** ASCII `M2M-MANIFEST-1\0` + sourceName str + releaseId str + archiveId str + sourceArchiveSha256 hex + coverageBbox 4×f64 + horizontalCrs str + verticalDatumStatus str + verticalDatumId str + nominalGroundSampleMeters f64 + nativeResolutionSource str + uncertaintyMeters tagged [u8(0) for null, u8(1)+f64 otherwise] + coverageStatus str + coverageIndexSha256 tagged [u8(0) null / u8(1)+hex] + sourceBytes u64 + licenseIdentifier str + attribution str + decoderVersion str. Excludes self `manifestSha256` and derived `localAssetToken` (the latter must match the archive SHA). Reference SHA `08f6fe9aa91e6415b162fc7b7f42616c302da72bf5fb946a03620c6627e121f3`.
- **Grid:** ASCII `M2M-GRID-1\0` + width u32 + height u32 + selectionId str + sourceManifestSha256 hex + horizontalTransformId str + verticalDatumId str + anchor lon f64 + anchor lat f64 + minEast f64 + maxNorth f64 + dx f64 + dy f64; then **each row-major sample**: u8(1)+finite elevation f64 when valid, or u8(0) only for false/null. Excludes itself, grid ID and transport token. Synthetic non-square signed ramp reference SHA `62ba45025ea9f6f4de60edcfa05136de7097c0a87ce77541322cb1a8bb83b7dd`; same ramp with one null cell reference SHA `8f9b1b045fa0f5c1d31da6f927a1da48c4e1e70d565763ecb37db5771aaff833`.
- **Canonical geometry:** ASCII `M2M-MESH-1\0` + vertex count u32 + vertices in lexicographic order of quantized XY Z signed i64 at **1e-6 mm resolution** (each value `sign(x)*floor(abs(x)*1e6 + 0.5)`, half away from zero) + triangle count u32 + triangles sorted by UTF-8 partId then lexicographically by oriented vertex indices. Triangle indices are remapped to sorted vertices; orientation cannot be reversed, only cyclically rotated so smallest index comes first. Each triangle: partId str + 3×u32. Equal quantized vertices or degenerate indices are invalid. ZIP/3MF compression/timestamp is **not part** of this hash. Reference outward-oriented 10×20×30 mm box has SHA `19755deb6cdd855f7a5facb8067fbbdec6a7667378f22a9acda85c1854cfa9ac`. Cross-language tests required.

The V1 binary grid transport format is fixed below. `gridDataSha256` hashes **that entire transport file** (distinct from canonical `gridSha256`). Agent 2's native import registry must authorize and atomically register these verified bytes.

### C-04 Printable terrain scaling

`TerrainBuildRequestV1.selectionId` **must match** `ElevationGridV1.selectionId`, or reject with `HASH_MISMATCH`. `clipRectMeters=[minEast,minNorth,maxEast,maxNorth]` is a **closed** rectangle. G1 requires all four sides to coincide with grid **node coordinates** (within 1e-8 fractional grid steps) and include at least 2×2 nodes. Out-of-bounds or off-node rectangles fail `INVALID_DIMENSIONS`. Later versions may support interpolation via a new policy; G1 does not silently extrapolate. Missing samples inside the clip fail `DATA_MISSING`.

Define `S=modelWidthMm/(maxEast-minEast)`, and require `S == modelHeightMm/(maxNorth-minNorth)` to relative 1e-9: no XY distortion. `x_mm=S*(x-minEast)`, `y_mm=S*(y-minNorth)`. `elevationZeroMeters` must be <= the minimum valid included elevation. `z_top_mm=baseThicknessMm + S*verticalExaggeration*(z_m-elevationZeroMeters)`. The flat bottom is z=0; sidewalls join at the sampled contour edge. This guarantees **minimum** base thickness >= `baseThicknessMm`, even for negative source elevations. Baseline above the terrain minimum is rejected, not silently shifted. G1 may have a larger base than minimum. All geometry is mm.

Synthetic 4×3 grid extends 36m East ×16m North (dx=12m,dy=8m); output 180×80mm with S=5 mm/m, base=4mm, exaggeration=2, zero=-8m. Recorded heights -8, -5 and +12m yield top Z 4, 34 and 204mm respectively. With base bottom at 0, expected outer print bbox `[0,0,0,180,80,204]`.

### C-05 Mesh QA and 3MF provenance

`MeshAnd3mfResultV1` represents a **successful, validated** export only. It must report physical mm AABB, connected component and part counts, volume, closed/oriented manifold and degenerate counts, intersection check and validation diagnostics. Only validated mesh+package may receive a success artifact. Failure returns `JobStateV1.status="failed"` and **no export result**. No output publication for partial/cancelled jobs.

G1 targets **3MF Core 1.4.0** with `unit="millimeter"`, standard right-handed coordinate convention and a model at `/3D/3dmodel.model`; attribution/provenance JSON at `/Metadata/map2model.provenance.json` containing manifest/hash, engine build/version, selected bounds, export profile, source licensing, warnings, and canonical geometry hash (but **not its own digest**). `provenanceSha256` is SHA-256 of exact embedded provenance bytes, generated deterministically with its own defined canonical encoding in A3-02. `artifact.sha256` covers the **final 3MF ZIP bytes** independently of geometry hash. Include conformance/validator ID/version and diagnostics. Agent 3 to validate whether embedded custom relationship/part packaging and Core 1.4.0 compatibility are supported by chosen lib3mf build; the dependency is BSD-2-Clause-licensed but packaging and transitive terms require separate review. Actual two-slicer checks belong to G3.

References: https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md and https://github.com/3MFConsortium/lib3mf .

### C-06 Jobs, errors and cancellation

Allowed state transitions: queued→running|cancel_requested|failed|cancelled; running→running|cancel_requested|succeeded|failed|cancelled; cancel_requested→cancel_requested|cancelled|failed. Terminal succeeded/failed/cancelled have **no** outbound transitions; progress [0,1] never decreases and succeeded requires 1.0, `phase="complete"`, `reasonCode=null`. Failed requires a typed non-null reason (not `CANCELLED`); cancelled requires `phase="cancel"`, `reasonCode="CANCELLED"`. Cancellation acknowledgement is `cancel_requested`, not an export. Terminal artifacts may only be committed by an atomic operation after success validation. On cancel or failure, remove staged files and preserve previous verified outputs. Details are safe structured `retryable`, capped missing tile IDs, invalid grid index, invalid mesh element index.

CLI/IPC control messages remain <=1 MiB JSON lines; bulk grids travel via **native verified tokens**. The current G0 CLI is only a health/unsupported-op stub. The future sidecar must enforce maximum message length during **reading**, prevent arbitrary renderer paths, and define job cancellation/streaming messages before G1 integration.

### Outstanding approvals / G0 decision

- **Agent 2:** Confirm or amend local WGS84 linearized transform accuracy bound and datum policy, immutable index format and verified coverage, grid binary asset registration and hash/reference vectors.
- **Agent 3:** Confirm or amend canonical mesh ordering/quantization and hash vector, closed-grid clipping and base Z formula, 3MF version/validator plan, result and job error details.
- **Lead:** Implement reusable Rust validator / digest parity; ensure TS/Rust/JSON mirror alignment and CI with negative fixtures.
- **Product owner:** Explicitly accept G0 frozen contract commit and source-policy limitations **after both reviewers sign off**. A2/A3 implementation must remain blocked until then.

This ADR is a specific **revision proposal**, not an assertion that the two other agents have already approved these choices.

## Agent 2/3 second-review resolution (proposed, not accepted)

**Source:** comments on PR #11 reviewing SHA `235351f9fabf10306e93898b7517c6cd7b4fc6c8`. The following changes are proposed as the lead's response. G0 remains blocked pending both agents' explicit acknowledgement and product owner authorization.

**Token vs validated grid:** `storageMode="local-binary-token"` is a **pending control envelope**, never a ready grid. Both JS and Rust reject it for direct geometry with typed `DATA_MISSING`. An authorized native resolver must read bytes, verify the file's transport SHA-256, decode the binary representation, then recompute canonical grid SHA-256 before passing an internal decoded grid to the build engine. JavaScript `hydrateGridFromVerifiedRegistry` and Rust `hydrate_verified_grid` implement equivalent reference behavior. No renderer-supplied filesystem path, unknown URL, or token string is sufficient to authorize a grid.

**Frozen G0 binary layout:** header = 15 bytes ASCII `M2M-GRID-BIN-1\\0`, then three unsigned little-endian 32-bit integers (width, height, node count). Require count=width×height, dimensions 2..4096 and count≤1,000,000; reject invalid/truncated/trailing bytes. Each north-up row-major sample consists of u8 flag 0 for missing (no following bytes) or flag 1 plus finite IEEE-754 f64 little-endian meters (normalize -0 to +0). Max file length 9,000,027 bytes. Transport checksum includes **all header and sample bytes**. The separate canonical grid hash also includes grid coordinate/provenance metadata as already specified. Binary fixture `ElevationGridV1.valid.bin`: 135 bytes, SHA-256 `f5e61b6c143b3a3d7027eae439b37f4450f53eeaa8ec853de157486d4b69ae4e`; missing-cell fixture: 127 bytes, SHA-256 `a53e70baa44e52c0e1af3a81242c9cc78f55604748bd76c406a6a0edc05e8955`. A decoded grid is an *internal engine representation*, not a 1 MiB-limited JSON IPC message.

**Ingress:** Both consumers must validate schema discriminants and fail-closed G1 policy `noDataPolicy="fail"`, spacing 0.1..2000 m, sample budget and bbox limits before expensive work. The Rust WGS84 transform rejects out-of-range longitude anchors in addition to unsupported latitude. Unknown datum/projection is not silently substituted.

**Coverage:** Neither a PMTiles header nor a manifest's `coverageIndexSha256` declaration is proof the required Z/X/Y tiles exist. `validateManifest` returns `indexDeclared` only (Rust `validate_manifest` has no coverage-ready return). Agent 2's native registry must verify exact index bytes and per-selection tile presence before offering a separate Generation Ready outcome; forged-index tests assert that metadata-only validation never reports Ready.

**Projection error budget:** `WGS84-ENU-LINEARIZED-V1` remains first-order local East/North, now restricted to rectangular selections ≤2 km per axis within ±60° latitude, with horizontal edge discrepancy ≤0.1% against WGS84 geodesic. Independent GeographicLib/WGS84 inverse reference: anchor [0°,59.98°], local 2000 m East/North corner [0.03582067436495238°,59.99795139610392°]. Its top East edge is 1998.9172358132557 m geodesically; west North edge 2000.0027391521326 m (errors 1.083 m and 0.003 m, both <2 m). An external projection or a new version is required for larger regions/polar use. Agent 2 should review whether this accuracy bound is suitable for terrain-printing applications.

**Geometry-result linkage:** A successful export is never validated in isolation. `validateResultAgainstRequest(result,request,grid?)` and Rust `validate_result_against_request` verify jobId and exportToken; measured print bounding XY min [0,0], requested width/height within absolute ±0.05 mm; minimum measured thickness ≥max(baseThicknessMm,minimumFeatureThicknessMm) within 0.05 mm; and optionally grid-derived maximum Z (±0.05 mm). The standalone canonical 10×20×30 mm box **hash fixture** is separate from the synthetic successful result envelope representing a 180×80×204 mm terrain print; neither constitutes an actual exported 3MF.

**Important future work (before G1 production):** The native registry must implement atomic authorization and verified index coverage queries; geometry/output staging must remain crash/cancel-safe; two slicers and official Core conformance are G3 gates. Native validators currently establish the G0 boundary contracts and synthetic fixtures, not actual source/mesh/export functionality.

## Agent 2 final C-02 manifest ingress correction

**Status: implementation proposed, pending Agent 2's approval of exact amended commit.** The canonical manifest digest intentionally excludes `schemaVersion` and `kind`; therefore a valid digest **cannot** replace runtime discriminant checks. The JSON Schema's const/version and semantic source rules must be enforced in both Node and Rust, including when Rust merely deserializes an untrusted JSON object without AJV.

`validateManifest` and `validate_manifest` now require exactly `schemaVersion="1.0.0"` and `kind="TerrainSourceManifestV1"`; allow `coverageStatus` only `"envelope-only"` or `"verified-tile-index"`; require mandatory nonempty source ID/release/archive, horizontal/vertical references, native-resolution provenance, license/attribution and decoder identifiers; and reject nonpositive, nonfinite or over-maximum source sizes and nominal sample spacing. `sourceBytes` must be a positive integer <= 2^53-1 so it survives the JavaScript/Rust boundary exactly; `uncertaintyMeters` must be null or a finite nonnegative number. Required SHA-256 identities remain lowercase 64-hex. A declared index must contain a well-formed immutable digest but **must not imply real coverage or generation readiness**.

Cross-language negative tests explicitly mutate only the excluded `schemaVersion` or `kind` while preserving their otherwise valid manifest digest, and recompute the manifest digest after introducing an invalid `coverageStatus`. Both language validators must reject those forged records before a materialized geodata asset can be used.

This completes the lead's implementation response to the remaining C-02 issue. **The G0 gate does not close until Agent 2 signs off and the product owner accepts the freeze.**
