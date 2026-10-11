# A3-02 Native API and Lifecycle Handoff

**Agent 3 → Agent 1 / Electron native worker**. Scope: frozen six G0 V1 contracts, deterministic synthetic/decode-grid-to-mesh-to-3MF only. Lead owns actual native application integration, on-disk verification, source authorization, and terminal job lifecycle.

## Owner Windows preview evidence

Product owner confirmed that the v0.0.2 Windows GUI's SYNTHETIC 3MF export loads and **slices in both PrusaSlicer and Bambu Studio**. Basic dual-slicer smoke is accepted. Neither the standalone Agent 3 CI ZIP, exact slicer versions, dimensions/tolerance, repair warnings nor network disconnection were separately attested. Do not request the same basic smoke again or claim full 3MF Consortium package conformance.

## Stable public Rust entry points

- Map2Model engine: `map2model_mesh::build_mesh(&ElevationGridV1, &TerrainBuildRequestV1) -> Result<BuildOutput, MeshError>`.
- Exporter: `map2model_export::package::export_3mf(&BuildOutput, &ElevationGridV1, &TerrainBuildRequestV1, &TerrainSourceManifestV1) -> Result<Exported3mf, ExportError>`.
- Publisher: `map2model_export::package::write_3mf_atomic(destination: &Path, artifact: &Exported3mf, is_cancelled: impl FnMut() -> bool) -> Result<(), ExportError>`.
- Frozen job-code adapter: `ExportError::job_reason_code() -> &'static str`; never copy internal `ExportError.code` directly into a wire `JobStateV1`.

The native worker must accept the grid **only** from Agent 2's verified offline decode or an explicitly labeled original synthetic-test-fixture. Do not trust renderer-provided archive files/paths, grid samples, arbitrary operations, URLs, or destination paths. The native Save dialog/project store owns the destination. A canonical manifest or source archive SHA is not independent proof of a real source's rights/datum, and an envelope-only synthetic manifest is not production coverage evidence.

## Lifecycle boundary

1. Lead authenticates/bounds native input; Agent 2 resolves offline verified source token and materializes `ElevationGridV1`; validate grid/source/selection and frozen hashes.
2. In a job in mesh phase, call `build_mesh`. The engine handles variable node-aligned rectangular grid shapes, full or interior clip, vertical exaggeration, minimum print thickness, and 100k clipped-node resource limit. No assumption of a fixed 4×3 fixture.
3. In export phase, call `export_3mf`. It preflights source linkage/license, known synthetic or authorized real datum, print bounds, actual manifold mesh, checks embedded OPC/provenance/mesh identity and returns **in-memory bytes**, never job success.
4. In verify phase, call `write_3mf_atomic` with the native-authorized output path and a cancellation callback. It checks cancellation, allocates per-attempt OS-random 128-bit exclusive stage, writes+syncs, rechecks cancellation, then atomically no-clobber links to destination. It removes only its own temporary file on failure.
5. **Lead must reopen final published file, verify file SHA, and only then** validate and emit `MeshAnd3mfResultV1` and a terminal `JobStateV1` success. Atomic hard-link publication is the commit/linearization point. Cancellation racing *after* publication and worker crash/timeout/permission failures are resolved by the Lead's authoritative state machine, never by an invented successful wire message. A cancelled terminal state must not silently leave an orphan published artifact.

ExportError mapping: `INVALID_EXPORT` and unknown private codes map to existing frozen `EXPORT_FAILED`; `CANCELLED` maps to `CANCELLED`; `DATA_MISSING`, `INVALID_GRID_HASH`, `INVALID_SOURCE`, `UNSUPPORTED_DATUM`, `HASH_MISMATCH`, `INVALID_DIMENSIONS` and other allowed frozen codes retain their meaning. See `tests/error_mapping.rs` verifying every exported code against `JobStateV1.schema.json`. MeshError is adapted via `ExportError::from` or translated to an allowed native code.

## Hash domains and provenance

- grid_sha256: canonical frozen V1 decoded grid samples/masks+selection+source identity.
- source manifest SHA: frozen source declaration identity; NOT independent redistribution or datum approval.
- canonical_geometry_sha256: normalized quantized physical geometry identity.
- provenance_sha256: exact JSON bytes at /Metadata/map2model.provenance.json, including source identity, rights declaration, datum, exaggeration, dimensions, mesh identity.
- package_sha256: actual completed .3mf ZIP bytes, which Lead must compare to published on-disk bytes.

Do not conflate these SHA domains. An identical mesh may have different provenance and package SHA if source identity or other metadata differs.

## Integration tests and next cross-agent seam

`crates/export/tests/varied_grids.rs` tests three original **decoder-shaped** synthetic grids of different sizes and vertical exaggeration, node-aligned interior clipping, positive volume, variable mm dimensions, repeatable ZIP/provenance/mesh hashes, independent Python ZIP/XML/geometry reader and fail-closed missing samples, altered grid hash, unknown source datum, missing license, selection mismatch and cancellation. These heights were generated in the Agent 3 test: **they are not actual decoded 512px WebP / PMTiles samples**. Do not claim the entire PMTiles pipeline already passed.

Agent 2 owns an original, redistribution-safe real 512×512-WebP PMTiles fixture and native decode contract. Lead must test that exact decoded `ElevationGridV1` through the public exporter functions on their temporary integration branch, with synthetic-only labels and no network. Real Interlaken data remain private and unauthorized for production.

## License/NOTICE integration for Windows

The existing CI `cargo_license_texts.py` captures verbatim license/NOTICE texts and per-file SHAs from exact resolved crates. The additional `notice_shipping.py` selects explicitly reviewed candidate SPDX alternatives and exposes commands:

~~~sh
python crates/export/tests/cargo_license_texts.py
python crates/export/tests/notice_shipping.py stage SOME_STAGING_DIRECTORY
python crates/export/tests/notice_shipping.py verify SOME_STAGING_DIRECTORY
~~~

The staged `resources/legal/THIRD_PARTY_NOTICES.md` and `THIRD_PARTY_NOTICES.manifest.json` must be included **unchanged** inside the actual Lead-built Electron/Windows distribution; `verify` checks both exact files, chosen package expressions and Cargo.lock SHA, and CI checks that tampering fails. The lead must separately collect Electron/Node notices and test the actual packaged EXE/resource extraction. A passing Agent 3 **test staging directory** is not a shipping EXE or human legal signoff. Explicit Lead license-policy and release review remain required.

The exporter and its draft PR must not be independently merged; Lead controls final branch integration and release gates.
