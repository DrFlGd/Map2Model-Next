# A1-04 integration dispatch — latest agent updates and lead execution

**Issued by Agent 1 (orchestrator/integrator):** 2026-10-10, after re-review of A2-01/A2-02 and A3-02.
**Status:** ACTIVE; instructions and evidence gates, **not a successful Windows integrated build**. Frozen G0 V1 schemas stay unchanged.
**Priority:** get an **offline synthetic grid -> real mesh -> verified 3MF** working in one Windows build before representing real Mapterhorn as production-ready.

## Current exact implementation checkpoints (do not use older handoff SHAs)

| Team | Work/PR | Head under review | Verified evidence | Disposition |
|---|---|---|---|---|
| Lead | [A1-03 desktop shell PR #12](https://github.com/DrFlGd/Map2Model-Next/pull/12) | `12e48646861a33f7ba5df06e3392861506e80008` | owner reports successful manual Windows shell use; **network disconnected and save/reopen not yet explicitly attested** | draft, UI not wired to native jobs |
| Agent 2 | [A2-01 importer PR #14](https://github.com/DrFlGd/Map2Model-Next/pull/14) | `89478557aceadcc54df871f6e3f8858188bcec4c` | [CI 38093164760](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38093164760): Windows+Ubuntu pass | importer subset code-level accepted; NOT real-source production acceptance |
| Agent 2 | [A2-02 stacked decoder PR #16](https://github.com/DrFlGd/Map2Model-Next/pull/16) | `1a0fbd3403144dd9ef2dc1199eb2cf7f3ccf9a91` | [CI 38103209348](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38103209348): Windows+Ubuntu pass; native synthetic grid->mesh | code-level synthetic path accepted; stacked, unmerged |
| Agent 3 | [A3-02 exporter PR #15](https://github.com/DrFlGd/Map2Model-Next/pull/15) | `7b2f27e94308e5613e64dca15851ec746ae2d552` | [CI 38099058321](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38099058321), [real Rust 3MF external validators 38099058331](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38099058331): Windows+Ubuntu pass | actual writer exists; Windows GUI slicers NOT tested |
| Lead | G0 and A3-01 merged; docs handoff merged | `main` contains A3-01 | frozen schemas, mesh generator, tests | foundational |

**Lead review comments:** [importer](https://github.com/DrFlGd/Map2Model-Next/pull/14#issuecomment-6104394745), [decoder](https://github.com/DrFlGd/Map2Model-Next/pull/16#issuecomment-6104394496), [exporter](https://github.com/DrFlGd/Map2Model-Next/pull/15#issuecomment-6104394292).

## Agent 2 — continue on existing PRs; no pause for A1-04

1. **A2-01 #3:** maintain exact-byte immutable registry, conservative 4-edge XYZ coverage, strict source declaration, no auto-network; root Cargo conflict already resolved and CI green. Keep PR #14 draft pending production source acceptance. Request re-review on every new exact head.
2. **A2-02 #4:** finish private **native Rust** `terrain_offline inspect` on the corrected Interlaken sample, with source SHA and RGB/Terrarium pixel cross-check; format-only inspection must retain `productionGenerationReady=false`, unknown vertical datum. Existing CLI and synthetic 512px grid-to-mesh smoke already work; do not redo them.
3. **Corrected private Interlaken extract** (owner-reported, not published): `interlaken-z17.pmtiles`, 3,127,295 bytes, SHA-256 `75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0`. `pmtiles verify` reportedly succeeded, 40 WebP tiles 512x512 RGB at z13–17. Original `interlaken-test.pmtiles` SHA `40f452bd2449ac0b119efd1cabfcd1a0937d375865e4795810bf82d4743ad5ce` differs at **one byte** (PMTiles offset 101; maxZoom 18 -> 17). Record both identities and derivation; do not silently use the original invalid-header identity.
4. Actual regional PMTiles metadata is **attribution-only** and has no `encoding=terrarium` field. The current strict production importer rejects it; format-only inspector rightfully does not authorize generation. Propose a **lead-reviewable, independently documented encoding attestation bound to exact archive SHA**, without changing G0 schemas or weakening unknown datum checks. Real-source datum, license/derivative rights, contributing DEM provenance, pinned upstream release and native accuracy are separate, currently **unverified** gates.
5. Produce test output/error evidence, command/version/SHA and no-network/native Windows status on [issue #4](https://github.com/DrFlGd/Map2Model-Next/issues/4). Do not upload unlicensed terrain bytes into this public repository, claim `Generation Ready`, or independently merge stacked PR #16.

## Agent 3 — continue on PR #15; no pause for A1-04

1. Current head emits **real** deterministic 3MF with independent Python ZIP/OPC verifier, `trimesh` import, and published Core 1.4 **model XSD** checks. Do not revert to preflight-only. The XSD workflow transparently adjusts 16 `maxOccurs` values only in its parser; it is **not** whole-3MF Consortium conformance.
2. Windows synthetic artifact: [run 38099058331](https://github.com/DrFlGd/Map2Model-Next/actions/runs/38099058331), `a3-02-rust-3mf-windows-latest`; Rust ZIP SHA `8463789ff97816d3dccd8b8085444c623150fb68417f3da04eafe9364afce675`; provenance SHA `6f4536749caa1a2b1cccda07b62b339eb70bd04d77c770ff5e0d4f1d8de66d07`; mesh SHA `dd8c9fa5c7734534d176ef02ee38299b5911343d7112933298dd6dfc79185db9`. Physical envelope `180×80×204 mm`. Only **actual Rust-generated file** counts for slicer acceptance.
3. Finish manual Windows **PrusaSlicer and Bambu Studio** independent import **and slice**, record versions, file SHA, offline state, dimensions ±0.05 mm, repair warnings, screenshots/layer previews. [Test procedure](https://github.com/DrFlGd/Map2Model-Next/blob/agent/A3-02-3mf-export/crates/export/SLICER-ACCEPTANCE.md).
4. Fix `write_3mf_atomic` staging pathname so retries after abandoned stage succeed (per-attempt randomized exclusive filename; concurrency/cancel/no-clobber regressions). Map native exporter `INVALID_EXPORT` internally to existing frozen `JobStateV1.reasonCode=EXPORT_FAILED` in the lead supervisor; do not change V1.
5. Produce transitive NOTICE/license text for distributed Rust dependencies, not only an SPDX inventory; coordinate the lead-owned CI workflow. Keep #6 and draft #15 open pending slicer evidence, notices, error mapping and native job lifecycle verification.

## Agent 1 — actual A1-04 implementation path

**Owned:** `apps/desktop/`, `crates/engine-cli/` (lead native host), `docs/integration/`, integration tests, shared build configuration/CI. Preserve other agents' ownership.

**Pass A — native host hardening on top of merged A3-01 `main`:**
- Harden `map2model-engine --stdio` from unbounded `BufRead::lines` to bounded per-message stream framing *before allocation*, with max 1 MiB frame and deliberate invalid UTF-8/oversized behavior, request ID policy, stable typed errors and multi-frame resync tests.
- Add a trusted **synthetic mesh smoke** using frozen G0 grid/request fixtures and the actual `map2model-mesh` build. Report true vertices, triangles, AABB and canonical mesh SHA. Never announce real generation or 3MF from the mesh-only host.
- Keep engine process running only in Electron's native main/preload boundary; never execute renderer paths, arbitrary operations, scripts, remote URLs or unbounded grid JSON.

**Pass B — merge or compose reviewed components into a temporary integration branch without falsely closing their issues:** A1-03 Electron UI; A2-01 importer & stacked A2-02 decoder; A3-02 exporter. Don't merge their draft PRs to main before their acceptance gates. Reconcile Cargo workspace `engine-cli/contracts/engine/data/export` and test exact integrated SHA on Windows+Ubuntu.

**Pass C — native-only stateful job supervisor:** explicit allowlisted operations for a *synthetic* grid build first, source-token validation for real data later; package engine binary in Electron executable; authenticate the IPC origin; bound stdio and file sizes; derive export destinations from native chooser/project root only; report frozen `JobStateV1` (progress, cancellation, reasons) and successful `MeshAnd3mfResultV1` **only** after verified 3MF atomic publication. Secure launch/no-network, no arbitrary renderer file paths, no misleading `Generation Ready` on unverified sources.

**Pass D — CI and owner smoke:** synthetic `ElevationGridV1` -> watertight mesh -> **real** deterministic 3MF -> slicer offline; then separately permit verified real PMTiles source -> decoded grid -> mesh -> 3MF. Capture Windows EXE artifact SHA, create/reopen project, cancel/failed jobs, provenance, final file SHA, and M1-AT-01..08.

## Product owner / operator action (parallel)

- Clarify whether A1-03 shell test included **airplane/network-disabled mode** and a saved project reopened after restart; record as PR #12 evidence.
- Test `synthetic-terrain.3mf` from Agent 3's **Windows artifact** in **PrusaSlicer and Bambu Studio** (same exact unmodified ZIP SHA, one mesh, 180×80×204 mm, actual slice, warnings/screenshots). This helps unblock #6 without another installer.

**Dispatch rule:** Agents 2 and 3 may continue independently immediately; the lead proceeds with A1-04 in parallel. Tests/provenance are accepted only at exact head; no silent G0 mutations, no source/license/datum inventions, and no premature merge/release.
