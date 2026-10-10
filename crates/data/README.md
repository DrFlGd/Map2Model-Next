# A2-01 — Offline PMTiles terrain importer

**Status:** Reviewable G1 Rust implementation. This crate is not wired into Electron/IPC and does not produce ElevationGridV1 or actual 3MF. Frozen schemas and types are unchanged.

## Supported archive profile

- Local PMTiles v3 with 127-byte header, root directory inside first 16 KiB, varint directory, and optional one-level leaf directories. Nested leaves rejected.
- Tile type WebP, metadata encoding terrarium, tile and internal compression none or gzip. Brotli, Zstd and unknown codecs explicitly unsupported. Pixel/image semantic decoding is A2-02.
- Referenced WebP payloads are checked for RIFF/WEBP signature and RIFF size, plus bounded decoded bytes. Does not yet certify 512×512 images or Terrarium RGB decoding.
- Bounded: archive 128 MiB, decoded tile 2 MiB, 200,000 indexed IDs, directory inflation 16 MiB, metadata 128 KiB, zoom ≤22, header bbox non-wrapping and within ±60° latitude. These are G1 subset restrictions, not PMTiles v3 format limits.
- Tile presence verified independently against actual PMTiles directory records using Hilbert Z/X/Y IDs. Missing Z/X/Y returns DATA_MISSING; never infer tile availability solely from archive header envelope.
- Source archive SHA-256 hashes exact archive bytes. Manifest SHA uses the frozen V1 canonical::manifest_digest. Immutable index SHA uses ASCII magic M2M-PMT-INDEX-1 followed by NUL, raw archive SHA bytes, little-endian u32 tile count and sorted (tileID u64, absolute offset u64, encoded length u32) records.
- Native content-addressed asset/index storage; m2m-asset and m2m-index tokens are not arbitrary file paths. Each tile read checks immutable source digest again.
- No networking, download, interpolation, secret source fallback, or datum conversion.

## Source rights and vertical reference

Import requires source/release/archive IDs, explicitly documented region license and attribution, positive nominal sample spacing with provenance, and verified known vertical datum ID. None may be guessed from zoom, viewer software licensing or generic provider attribution. Unknown/unsupported vertical datum returns UNSUPPORTED_DATUM. Lawful redistribution and derivative permissions must be independently evaluated for each real extract.

References: https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md ; https://mapterhorn.com/data-access/ ; https://mapterhorn.com/attribution/

## Test and acceptance evidence

Run: cargo test -p map2model-data. Workspace CI exercises crate tests on Ubuntu/Windows. Synthetic in-memory PMTiles fixture has actual directory entries for two distinct XYZ tiles and original 1×1 lossless WebP files generated solely for framing/integrity tests. These are **not** purported Mapterhorn data or 512×512 DEM tiles.

Negative tests include bad magic/version, unsupported tile type, unsafe offsets, directory corruption, missing tile, unknown datum, checksum failure and registered archive mutation. Follow-up needed for full A2-01 G1 acceptance: a legally obtained version-pinned actual Mapterhorn subset; real source datum and region permissions; gzip/leaf fixture coverage; Windows file-system/read-only smoke; and native integration.

Ownership is Agent 2 in crates/data. The root Cargo.toml workspace-member addition is expressly submitted for lead review so the existing CI actually runs data tests.


## Follow-up to lead review (PR #14)

- The Rust API now exposes `reopen_registered_pmtiles(registry_root, saved_manifest)`, which verifies the V1 manifest, exact immutable archive hash and byte count, scans the PMTiles directories again, and independently authenticates the registered index bytes and its digest. No retained in-memory index is needed to resume after restarting Windows.
- `verify_required_tiles` revalidates the disk index on each lookup. `verify_selection_coverage(selection, zoom)` checks the frozen V1 geographic selection, archive envelope, requested zoom and every intersecting XYZ tile (bounded to 4,096 required tiles); a success is an actual index lookup, never manifest-only readiness. Engine/IPC integration is still not implemented.
- One-level leaf directories are only accepted when the first child tile ID matches its root parent and each child's run is before the next root key. Synthetic gzip fixture exercises independently gzip-compressed root, metadata, leaf and tile bytes; negative leaf-parent/overlap and malformed gzip cases fail closed.
- Native asset registration uses 16 bytes of OS entropy for an exclusively created random staging filename, atomic hard-link publication, checksum verification and readonly final path. Crash-leftover and concurrent import tests ensure no PID-based stage collisions. Bounded pre/post metadata and handle reads avoid unbounded memory growth if a file changes during import. Native registry root must be a permissioned app-controlled path.
- No implementation can independently assert source-region rights or vertical datum from the generic PMTiles header. A lawful, release-pinned real Mapterhorn example with documented rights, source datum and native resolution is required before acceptance; 1×1 WebP fixture is only a framing test, not real Terrarium elevation.
