# A3-02 — Real deterministic 3MF Core 1.4.0 exporter

**Status:** Actual ZIP-based 3MF writing, strict internal OPC/geometry/provenance readback, and no-clobber atomic publication are implemented in the **draft PR #15**. **Published Core 1.4 model-XSD verification, independent OPC/mesh validation and third-party trimesh import have passed on the real Rust ZIP; PrusaSlicer/Bambu Studio physical Windows open-and-slice acceptance, full production-OPC conformance and native job IPC remain unverified.** Issue #6; depends on A3-01 merged #13.

## Writer decision, third-party licensing and review

- **Writer/package:** pure Rust `zip = 2.4.2`, MIT license; default features disabled, ZIP `Stored` for cross-platform deterministic byte output and no compressor dependencies.
- **Independent XML parsing:** `roxmltree = 0.20.0`, MIT OR Apache-2.0. DTD/entity expansion disabled by default, namespace-aware OPC and Core XML validation. ZIP and XML readback is implemented separately from construction; it is **not** a substitute for independent official conformance.
- `serde_json = 1.0.145` (MIT/Apache-2.0) and `sha2 = 0.10.9` (MIT/Apache-2.0), already in project use. Pinned exact versions in Cargo.toml; review transitive package notices and Cargo.lock before redistributing binaries.
- **Lead status:** pure-Rust dependency design provisionally approved for independent conformance testing, **not yet approved for merge/distribution**; see [lead handoff](https://github.com/DrFlGd/Map2Model-Next/issues/6#issuecomment-6103216160). Required final evidence includes Windows PrusaSlicer + Bambu Studio real import/slice checks, transitive license notices and native job lifecycle. Decision request: [issue #6](https://github.com/DrFlGd/Map2Model-Next/issues/6#issuecomment-6103049410).

## Real output and deterministic provenance

`export_3mf(&BuildOutput,&ElevationGridV1,&TerrainBuildRequestV1,&TerrainSourceManifestV1) -> Result<Exported3mf,ExportError>`:
1. `prepare_export` enforces original frozen V1 grid/request/source, verifies license/attribution, canonical mesh SHA, physical dimensions/print thickness, mm units, single closed and oriented mesh.
2. Create **real 3MF ZIP** with four OPC parts, in deterministic order with stable timestamp and Stored compression:
   - `[Content_Types].xml` (model, JSON, relationship MIME entries)
   - `_rels/.rels` (root StartPart and required MustPreserve for the custom JSON)
   - `3D/3dmodel.model` (Core 1.4.0 namespace, single mesh object, 3-axis mm vertices and outward-oriented triangles, build item; vendor-qualified source license, attribution, engine version and mesh/provenance hashes)
   - `Metadata/map2model.provenance.json` (deterministic UTF-8 source/license/datum/engine/geometry)
3. Independently reopen ZIP with `ZipArchive` and parse OPC and Core XML with `roxmltree`. Reject missing/extra parts, invalid types/relationships, non-mm units, malformed/out-of-range indices, invalid geometry, canonical SHA disagreement, missing/wrong provenance bytes, bounding mismatch, or other bad shapes.
4. Return **actual bytes**, `package_sha256` of final ZIP, `provenance_sha256` over the embedded JSON, and `canonical_geometry_sha256` over the frozen, quantized V1 mesh representation. These are separate SHA domains; an archive can vary without changing the geometry identity.

`write_3mf_atomic(destination,&Exported3mf,|| cancelled)` is a native-side helper that first checks cancellation and the artifact digest, stages into the requested directory, `sync_all`s, rechecks cancellation, and publishes using atomic **hard-link no-clobber** then removes the staging name. On error/cancel the staging file is removed, and an existing destination is not overwritten. Directory must be native-authorized and support hard links; no raw UI paths are accepted. A cancellation arriving **after** the link is published is a final-job-IPC race; the native supervisor must define the commit/success boundary and enforce state transitions. This API does not invent a V1 successful export result or claim third-party conformance.

## Reproduce a real slicer-importable candidate

```sh
cargo run -p map2model-export --example synthetic_3mf -- ./synthetic-terrain.3mf
cargo test -p map2model-export
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The example reads the unmodified frozen V1 synthetic grid/build/manifest. Expected **24 vertices, 44 oriented triangles**, AABB **[0,0,0,180,80,204] mm**, minimum 4mm base thickness and frozen canonical geometry SHA `dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9`. It writes a genuine `.3mf` file; run against a **nonexistent filename** in an existing directory (no-clobber).

`tests/actual_3mf.rs` performs ZIP/readback verification, verifies a 3MF's actual embedded licensing and provenance, checks byte-for-byte deterministic ZIP generation, deliberately corrupts OPC parts, vertex indices, unit and provenance, and verifies cancellation/atomic publish. Synthetic fixtures are original Map2Model Next artificial CC0-1.0 data; no proprietary or downloaded geodata.

## Actual independently verified Rust-generated sample

[Actions run 38098914848](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38098914848) generated real `.3mf` files on both Ubuntu and Windows and retained separate OS artifacts. Both runs passed the Python stdlib ZIP/OPC/directed-edge/geometry-SHA checker, the external `trimesh 4.11.1` 3MF importer (one solid, watertight, correct winding, 180×80×204mm, volume 1,449,600mm³) and the published Core **1.4.0** model XSD using `lxml 6.1.1`. The official schema is preserved unchanged on disk; the checker documents the in-memory libxml2 `maxOccurs=2147483647` to `unbounded` normalization (16 occurrences), recording original and derived hashes. This is a model-XSD pass, not complete official package/slicer certification.

Exact Rust file digests: ZIP `8463789ff97816d3dccd8b8085444c623150fb68417f3da04eafe9364afce675`, provenance `6f4536749caa1a2b1cccda07b62b339eb70bd04d77c770ff5e0d4f1d8de66d07`, canonical mesh `dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9`.

**CI workflow (lead-owned addition for explicit review):** `.github/workflows/a3-02-3mf-evidence.yml` separately builds/archives a redistribution-safe signed synthetic ZIP and all reports. `tests/cargo_dependency_report.py` inventories actually resolved package SPDX declarations and the Cargo.lock checksum for the lead to review; this does not substitute bundled dependency license notices. For the physical Windows test procedure and explicit `not-tested` tracking, see [SLICER-ACCEPTANCE.md](SLICER-ACCEPTANCE.md).

## Remaining external acceptance

- The **published Core 1.4.0 model XSD** test and two independent ZIP/mesh readers now pass on real Rust output. Full official Consortium **test_suites** and application import/conformance acceptance remain separate checks; no claim of full 3MF conformance made.
- Open and slice in **PrusaSlicer and Bambu Studio** at two lead-approved, version-pinned Windows versions; record open success/no repair, dimensions within ±0.05mm, expected appearance/base, screenshot/log, and resulting slice correctness.
- Final native IPC, progress, atomic staged artifact generation with crash isolation and canonical `MeshAnd3mfResultV1` success **only after the lead-defined external validation gate**. No G3 pass claims yet.
- Only node-aligned G1 rectangular clips; 100,000 clipped mesh-node cap inherited from A3-01. Only single-part terrain meshes; writer excludes textures, materials, arbitrary mesh soups and unknown extension namespaces.
