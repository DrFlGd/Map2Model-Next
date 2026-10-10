# A3-01 — Synthetic terrain mesh engine

Independent Rust library (`map2model-mesh`) for G0-frozen `ElevationGridV1` + `TerrainBuildRequestV1` in `crates/engine/`.

Entry point: `build_mesh(&grid, &request) -> Result<BuildOutput, MeshError>`.
The response includes the **actual vertices and oriented triangles**, a `GeometryReportV1`, and the **frozen SHA-256 canonical geometry digest**. It intentionally does not create `MeshAnd3mfResultV1`, fabricate artifact hashes or publish 3MF.

## Algorithm and validation

1. Call native G0 `validate_build`: decoded grid validity, V1 input hash, selection identity, all-inclusive grid-node-aligned clip, aspect ratio, finite dimensions, no missing values, and minimum baseline.
2. Generate top vertices in north-up row-major order, `x=S*(east-west)`, `y=S*(north-south)`, `z=base+S*verticalExaggeration*(elevation-elevationZero)`, with `S=widthMm/(east-west)`. Boundary XY coordinates are set to the exact requested dimensions. Negative elevations are legitimate; base is Z=0.
3. Generate coincident XY bottom grid at Z=0. Use consistent top (+Z), bottom (-Z), and CCW perimeter sidewall (outward) winding. Each grid cell uses two top and two bottom triangles, each perimeter segment two side triangles.
4. Independently inspect exact structured heightfield triangulation, finite/positive top heights, strictly monotonic XY coordinates, bottom alignment, expected connectivity, triangle area, opposite directed edge pairs, positive signed volume, single connected component, bounds and minimum Z thickness. **Self-intersection is excluded by mathematical construction** when these invariants hold: disjoint XY interiors of top grid cells, no top/base collisions, and disjoint simple vertical perimeter walls. This is not a general arbitrary triangle-soup intersection detector.
5. Compute geometry SHA using **existing** `map2model_contracts::canonical::canonical_mesh_digest` (V1 oriented triangle canonicalization, 1e-6 mm quantization). No changes to frozen contracts.

## Reproducibility

`cargo test -p map2model-mesh`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Checked on both Windows and Linux in the repository's existing GitHub Actions CI.

Signed 4×3 fixture builds **24 vertices / 44 triangles**, AABB **[0,0,0,180,80,204] mm**, **4 mm** minimum thickness, **1,449,600 mm³** volume and SHA-256 `dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9`. Reference data in `crates/engine/fixtures/A3-01.reference.json`. The hash was independently precomputed from the frozen V1 encoder recipe and cross-checked by Rust regression.

Additional tests cover flat negative terrain, asymmetric signed slope/north-up orientation, node-aligned subclip, no-data rejection, unresolved token, incorrect hash/selection, invalid aspect/base/size, winding, surface folds, zero top height, missing triangles, nonfinite geometry and repeated deterministic builds.

## Boundaries and limitations

- Inputs are **decoded, authenticated** G0 V1 grid values, not unresolved `m2m-grid:` tokens. Agent 2's native registry owns token authorization, binary-hydration and actual terrain source provenance. Geometry does not depend on PMTiles or Electron.
- G1 explicitly supports only **grid-node-aligned axis-aligned clips**; no off-node interpolation, mesh decimation, erosion, arbitrary polygon cutting, terrain-hole repair, or native asset loading.
- Mesh generator's current conservative resource cap is **100,000 clipped grid nodes**, below the G0 wire maximum (1,000,000). Exceeding this fails `OUT_OF_MEMORY` before expensive mesh allocation; reconsider with lead for performance testing rather than changing frozen contracts.
- Self-intersection certification applies only to this exact grid heightfield construction. Do not use this inspector to bless arbitrary user-imported meshes.
- Geometric minimum thickness is the minimum vertical top-to-base clearance; true local wall/nozzle printability and slicing are A3-02/G3 concerns.
- A3-01 neither implements cancellation/persistent jobs nor 3MF; lead must define worker invocation, cancellation checks, progress reporting and atomic output publication before integration.
- No real imported geodata, third-party data or proprietary assets are present; all tests use attribution-safe synthetic G0 fixtures.

Scope: Issue #5, GEO-05/06/07, MESH-01/02/03/05 and NFR-07. Review and merge by lead only.
