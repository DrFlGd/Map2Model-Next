# A2-02 — Offline 512×512 Terrarium WebP → ElevationGridV1

**Status:** Draft implementation stacked on A2-01 PR #14 (not merged). Issue #4 cannot close until #3 is accepted and lawful source/datum validation and Windows offline integration are complete.

## Contract and reproducibility

- A2-02 uses the frozen, unmodified `ElevationGridV1` and `SelectionV1` Rust types and canonical grid hash. No G0 field/type changes.
- Uses pinned `image = 0.25.8` pure-Rust WebP RGB8/RGBA8 decoder. Validates **exactly 512×512** source raster, 2 MiB maximum compressed tile and bounded decoded pixel allocation.
- Reads real local PMTiles raster bytes only through the authenticated A2-01 `ImportedSource::read_registered_tile` API. A full verified XYZ tile coverage preflight, manifest validation and **known vertical datum** are mandatory. Missing tile or selected alpha-not-255 pixel fails `DATA_MISSING` instead of inventing a height; source-unknown datum fails.
- RGB Terrarium decoding: `R×256 + G + B/256 − 32768` meters. Negative elevations are valid, with source quantization of 1/256 m. Alpha < 255 means a no-data pixel; it may be elsewhere in the source image but **not anywhere selected by the output grid**.
- Each grid has a node at both WGS84 rectangle edges. Node count per axis: `ceil(projectedExtent / requestedSampleMeters) + 1`, bounded by 4096 per axis and one million overall. Actual sampling pitch is `projectedExtent/(numberOfNodes-1)`, no coordinate extrapolation. The grid is anchored to selection southwest, `minEast=0`, `maxNorth=projectedNorthExtent`; stored samples are **north-up, west-to-east, row-major**, as Agent 3 requires.
- Raster sampling is deterministic nearest-cell (not smoothing/interpolation) from XYZ tile WebP pixels. At an exact Web Mercator tile seam the grid sampler uses the east/south tile (positive tile index), but A2-01's closed-rectangle coverage preflight conservatively requires both touching neighbors. This avoids false Generation Ready on missing neighboring tiles.
- `canonical::grid_digest` over the resulting actual samples is checked by `validate_grid`; `gridId = grid-<first32hex>`. No output is marked ready when a tile is absent, corrupt, unrecognized, or alpha-masked.
- Decoder has an explicit 64-tile cache cap per selection, preventing unbounded WebP decoded raster allocations; errors use typed source/size/no-data codes.

## Synthetic acceptance tests

`cargo test -p map2model-data`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; CI is Windows and Ubuntu.

Tests programmatically encode **actual, independently generated lossless WebP 512×512 RGB/RGBA** tiles with known Terrarium pixels and construct a synthetic local PMTiles v3 archive. Golden RGB values include -123.5m (127,132,128), +100.25m (128,100,64), and -30m (127,226,0); verify north-up/west-east variation, valid masks, stable canonical hashes, wrong-sized-image rejection, transparent/no-data rejection, and budget limits. No real geodata, location-specific licenses, hidden downloads or on-demand network calls.

## Remaining integration gates

1. Real legally imported version-pinned Mapterhorn sample and recorded archive source SHA, license, attribution, native resolution and independently verified vertical datum.
2. Windows native bridge via lead-owned A1-04, with read-only token-only source access; test end-to-end offline selection→decoder→grid→Agent-3 mesh.
3. Optional future geographic seam interpolation/versioned resampling improvements would require dedicated golden tests; never silently alter the frozen V1 grid bytes/hash.
