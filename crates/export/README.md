# A3-02 — 3MF export gate (writer pending lead approval)

**Status: writer-independent preflight implemented; NO 3MF package is created yet.** This phase follows accepted/merged A3-01 #5 / PR #13. Issue: #6. No frozen G0 V1 contracts are modified.

Entry point: `prepare_export(&BuildOutput, &ElevationGridV1, &TerrainBuildRequestV1, &TerrainSourceManifestV1)`. It verifies all contract ingress, license/attribution/source identities, datum compatibility, genuine mesh topology/report, physical dimensions and canonical digest. It returns **UTF-8 deterministic JSON provenance bytes** for `/Metadata/map2model.provenance.json` and SHA-256 over those exact bytes. This **is not** a `MeshAnd3mfResultV1` or a publishable 3MF. It cannot claim a slicer/conformance success.

## Writer and validation decision requested from lead

- Preferred writer: 3MF Consortium **lib3mf 2.5.0**, BSD-2-Clause, with an audited C ABI binding, pinned redistributable SDK builds and NOTICE review for libzip (1.10.1), zlib (1.3.1), cpp-base64 (2.rc.08) and fast-float (6.0.0).
- Core **1.4.0**, `millimeter` and an untransformed single mesh object at `/3D/3dmodel.model`. XML+OPC custom part `/Metadata/map2model.provenance.json` requires root `MustPreserve` relationship, correct content type and deterministic bytes. **Verify lib3mf attachment support before approval**; do not silently post-edit a completed ZIP into invalid OPC.
- Writer shall reopen the package with an independent, fresh lib3mf reader in strict mode, inspect the actual triangle mesh and mm bounds, validate OPC content-types and relationships, and compare embedded provenance bytes to preflight. Independently compute `artifact.sha256` over the final 3MF ZIP, separate from the existing `canonical_geometry_sha256` over quantized ordered triangles.
- Official 3MF Consortium Core 1.4.0 conformance **test_suites v2.4.1** (suite 3 or suite 9 Core), plus Windows **PrusaSlicer and Bambu Studio** opening/slice checks with versions, screenshot/logs and no repairs.
- 3MF writer/staging integration must be native-side, bounded, path-safe, cancel-aware and atomic; only a successful independently verified export may publish a `MeshAnd3mfResultV1`. Cancelled/failed jobs leave no success artifact.

Lead decision remains outstanding at [issue #6](https://github.com/DrFlGd/Map2Model-Next/issues/6#issuecomment-6101611213). No lib3mf SDK or ZIP writer is bundled in this commit.

## Preflight guarantees and explicit boundaries

- The mesh is checked against original signed grid/request and source manifest with `validate_build`, `validate_manifest` and `inspect_mesh`. It re-hashes the **actual** vertices/triangles using the frozen V1 canonical algorithm; it will not trust the result's reported hash. Hashes bind source and geometry separately.
- Provenance includes selection/grid/source archive and manifest digests, actual source license/attribution, datum/CRS, selected clip, original engine/profile, actual measured dimensions and QA, and actual geometry hash. No circular provenance-hash field is embedded. JSON fields serialize deterministically with the pinned serde_json version.
- **A manifest envelope is not proof of covered tiles.** This writer-neutral function does not make a Generation Ready claim. The caller must obtain native verified source index/coverage evidence before publishing a production export.
- Export package SHA, Core conformance, slicer readback, signed artifact result, cancellation/job IPC and atomic filesystem publication are *not* yet implemented.
- Synthetic fixture is original artificial Map2Model Next data (license identifier CC0-1.0 as declared in frozen manifest). No real geodata is included.

Testing after PR CI: `cargo test -p map2model-export`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`.

Reference specification: https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md ; official test suites: https://github.com/3MFConsortium/test_suites ; writer: https://github.com/3MFConsortium/lib3mf .
