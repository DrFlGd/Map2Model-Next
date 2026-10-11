# G1 active handoff — Agent 2, Agent 3 and lead

**Issued:** 2026-10-10 | **Scope:** Map2Model Next Phase 1 | **Maintainer:** Agent 1 / integration lead

This handoff supersedes the "awaiting G0 approval" language in the original kickoff briefs. G0 is **approved and frozen** in `main` (foundation/contracts merge `4807a5b`). A3-01 (#5 / PR #13) is merged (`7180825`). **Do not reopen or silently edit the six V1 contracts.** Any interface change needs a versioned proposal, migration impact and cross-agent signoff.

**Read first:** [Requirements §21](../requirements/Map2Model_Local_First_Requirements_v1.1.md), [source policy](../licensing/SOURCE_POLICY.md), [real Mapterhorn sample protocol](MAPTERHORN-SAMPLE-PROTOCOL.md), [PR #12 Windows shell](https://github.com/DrFlGd/Map2Model-Next/pull/12), [PR #14 geodata](https://github.com/DrFlGd/Map2Model-Next/pull/14), [PR #16 decoder](https://github.com/DrFlGd/Map2Model-Next/pull/16), [PR #15 3MF](https://github.com/DrFlGd/Map2Model-Next/pull/15).

## Status and release discipline

- A1-03 desktop PR #12 is still a draft. Owner reports successful basic Windows-shell testing, but **network-disconnected startup and saved-project reopen evidence were not explicitly confirmed**; do not claim the complete M1-AT-01/06 checks passed. The existing GUI has no native terrain decoder or exporter.
- A2-01 importer PR #14 contains verified-directory lookup, persistent index/source reauthentication, one-level leaf/gzip tests, randomized atomic staging and native geographic coverage query. The lead accepted those four implementation findings. **It still lacks a lawful pinned real Mapterhorn extract with trustworthy region-specific licensing, datum and resolution evidence**, and its earlier exact-XYZ-tile-boundary regression and root-workspace merge conflict have **now been addressed** in head `89478557` with Windows/Ubuntu CI passed; that revision still requires lead review.
- A3-01 mesh engine is merged and deterministic. A3-02 PR #15 has progressed beyond preflight to an **actual** pure-Rust 3MF ZIP writer, internal readback, provenance, ZIP SHA, cancellation-safe staging and green cross-platform CI. **Independent 3MF Core conformance and real reference-slicer import remain unverified.**
- A1-04 integration (#8) and A1-05 release (#9) are not accepted. Synthetic, offline end-to-end comes before real-source release.

### Agent 2 — execute next

**Own:** `crates/data/`, fixtures/tests for geodata, A2-01 #3 and A2-02 #4. Use the frozen `SelectionV1` / `TerrainSourceManifestV1` / `ElevationGridV1` types.

1. **Request lead re-review of the completed patch** on [PR #14](https://github.com/DrFlGd/Map2Model-Next/pull/14) at `89478557`: closed WGS84 rectangle tile-boundary tests now conservatively include both touching tiles on west/east/north/south edges and fail if either required neighbor is missing. Confirm the new A2-02 decoder sampling policy remains consistent; don't reopen this item without evidence of a failing case.
2. The importer branch has **already merged A3-01 from main** and retains **`crates/contracts`, `crates/engine-cli`, `crates/engine`, and `crates/data`** in the Rust workspace; Agent 2 reports green cross-platform CI on the exact PR #14 head. Preserve this during the upcoming lead-controlled review/merge, verify the actual combined test suite again on the merge candidate, and do not self-merge.
3. Obtain/inspect a lawful **small, version-pinned** Mapterhorn-format extract by following [the sample protocol](MAPTERHORN-SAMPLE-PROTOCOL.md); use online acquisition only outside the offline application. Verify actual WebP pixel dimensions/encoding, zoom/directory coverage and source metadata; record bytes/sha/version. Do **not** check copyrighted sample bytes into the public repo without documented redistribution rights. Do not assert known vertical datum on provider URL or Terrarium encoding alone.
4. **A2-02 already has a stacked draft [PR #16](https://github.com/DrFlGd/Map2Model-Next/pull/16)** on Agent 2's importer branch, with actual synthetic 512x512 WebP decode, signed Terrarium sample tests, missing/alpha rejection, north-up output and frozen grid SHA. Request lead review of its exact tested head, then validate decoded samples against the genuine extract when provenance is established. **Do not merge PR #16 independently of A2-01 acceptance**; no invented height or implicit fallback.
5. Provide a CLI or native headless integration test demonstrating **local source -> selected tiles -> actual decoded `ElevationGridV1`**, with source and datum readiness explicitly reported. Do not couple to Electron or silently change V1.

**Report back in #3/#4 and PR #16:** exact branch/head, completed boundary/combined CI evidence, real source acquisition worksheet, verified vertical-reference evidence (or `UNSUPPORTED_DATUM`), decoder sample comparisons, blockers. Real source ambiguity is a hard gate to production "Generation Ready", **not** a reason to fabricate metadata.

### Agent 3 — execute next

**Own:** `crates/export/`, 3MF fixtures/tests, A3-02 #6. A3-01 has already merged: do not redo mesh implementation.

1. Continue [PR #15](https://github.com/DrFlGd/Map2Model-Next/pull/15) from its **current** head, which already emits real 3MF OPC ZIP content; do not regress to a preflight-only plan. Keep geometry, provenance, and exported ZIP hashes separate; preserve the frozen `/3D/3dmodel.model` and `/Metadata/map2model.provenance.json` paths, units in millimeters, and safe cancellation/no-clobber output.
2. **Lead provisional writer decision:** the narrow fixed-schema, pure-Rust `zip = 2.4.2` (default features off, stored ZIP) + `roxmltree = 0.20.0` approach is acceptable for independent validation, **not a conformance/merge approval**. Keep exact dependency versions, review lockfile/transitive license notices and verify source/tool versions before Windows distribution. lib3mf v2.5.0 may be used as an independent **reader/validator** without making it a runtime DLL dependency; disclose any SDK license/notices.
3. Supply independent **3MF Core 1.4.0** compliance evidence beyond the writer's own ZIP/XML readback: use the [3MF Consortium Core specification](https://3mf.io/spec/), [official sample/conformance suite](https://github.com/3MFConsortium/3mf-samples) and/or a separate supported 3MF parser. Demonstrate OPC content types, StartPart/MustPreserve relationships, object/build refs, triangle indices, provenance-preserve semantics, correct units/dimensions and no auto-repair.
4. Open a **Rust-generated** `.3mf` (not independently reconstructed equivalent bytes) in both **PrusaSlicer and Bambu Studio on Windows**. Record exact tested versions, import diagnostics, screenshots, visible orientation, measured dimensions within 0.05 mm, whether each slices and whether either auto-repairs. If a slicer is unavailable, say so; do not claim pass.
5. Document the CLI recipe (`cargo run -p map2model-export --example synthetic_3mf -- ./synthetic-terrain.3mf`), output ZIP SHA, provenance SHA and canonical mesh SHA; preserve synthetic 180x80x204 mm reference results. Attach a rights-safe synthetic 3MF as a CI artifact if feasible, **never an unapproved Mapterhorn extract**. Provide an integration handoff for native engine job lifecycle: complete/failure/cancelled, atomic publication, and frozen `MeshAnd3mfResultV1` only after verified success.

**Report back in #6/PR #15:** exact current head, official validation results, both slicers and versions, licenses/notices, remaining nonconformance or IPC blockers. Do not self-merge.

### Agent 1 — integration sequencing (owner/lead)

1. Re-review Agent 2's **already-submitted** coverage-boundary/workspace fixes at PR #14 head `89478557`, and stacked real 512px decoder PR #16; run full-workspace CI before accepting #3 or permitting the stacked #16 merge. Record source rights/datum uncertainty openly.
2. Review PR #15 using external 3MF evidence; accept the pure-Rust approach only if package conformance, notices and slicer checks pass. Control merges and no frozen V1 schema changes.
3. Complete #12 Windows shell smoke acceptance: clarify whether the owner ran **physically offline**, and record create/save/reopen, selection digest and blocked HTTP/DNS evidence before closing #7.
4. Build A1-04 native-only transport/worker bridge, bounded IPC with explicit authorization, job progress/cancel/error mapping, no untrusted filesystem paths, persisted source/grid/export tokens. **First executable acceptance target:** synthetic `ElevationGridV1` -> real mesh -> actual 3MF -> reference slicer, all offline. Next: real **verified** Mapterhorn source -> decode -> grid -> mesh -> 3MF.
5. Publish a Windows preview only when the executable actually exercises that path. A coordinate-only repackaging must not be described as the terrain-enabled build. Product owner performs physical Windows smoke; G3 needs M1-AT-01 through M1-AT-08 and explicit acceptance.

## Contracts, reproducibility and rights (all agents)

- G0 is frozen. Exact source/selection/grid/mesh/export IDs and canonical algorithms are authoritative; a SHA-256 of ZIP bytes does **not** equal the canonical geometry SHA.
- Explicitly distinguish **synthetic-only tests**, **real-data format validation**, **source/datum accepted for production**, **official 3MF conformance**, and **slicer tested**. Never collapse them into a single "green CI" claim.
- Keep geographic acquisition/networking outside the offline runtime; do not port the previous proprietary mirrored application's JS, Web Workers or bundled website into Map2Model Next. The private legacy repo was inspected only to locate *public* source endpoint conventions; official Mapterhorn documentation is the authoritative citation.
- No sample archive or derived elevation/3MF artifact in this public repo without region/source-specific reuse and distribution clearance. No direct production dependence on the old online cache.
- Each PR: issue/requirement IDs, exact head SHA, command + CI links for Windows/Ubuntu, deterministic fixture hashes, third-party license/attribution records, known blockers, and a request for lead approval.

**References:** [Mapterhorn data access](https://mapterhorn.com/data-access/), [Mapterhorn attribution](https://mapterhorn.com/attribution/), [3MF Core 1.4.0](https://github.com/3MFConsortium/spec_core), [3MF samples/conformance](https://github.com/3MFConsortium/3mf-samples). This document records instructions and evidence requirements, not verified real-source/slicer acceptance.
