# Agent 2, Agent 3 and Lead — private/local experimental terrain review

**Issued:** 2026-10-11 by the Lead at owner direction. **Status:** Review authorization, not production source approval. **Authoritative policy:** [PRIVATE-LOCAL-TERRAIN-POLICY-2026-10-11.md](../licensing/PRIVATE-LOCAL-TERRAIN-POLICY-2026-10-11.md).

The owner directs us to follow the earlier Map2Model-style **private-use, local model generation, user responsibility, attribution and disclaimer** strategy. **Unknown vertical datum, underlying contributors and source license must not be fabricated.** But those unknowns need not prevent a **clearly separated experimental personal-use** local preview when the user supplies an already acquired file, sees warnings and explicitly opts in. Private use never creates legal rights or overrides restrictions on copying, caching, derivation or attribution. The app and published releases contain NO user-provided real terrain bytes or real-derived models.

This dispatch **supersedes historical instructions requiring full independent real-datum/rights attestation before any local experimental model**; those strict gates remain in force for verified production Generation Ready or distributing third-party real datasets and derivatives.

## Already completed — do not repeat

- Windows [v0.0.4 original PMTiles hotfix](https://github.com/DrFlGd/Map2Model-Next/releases/tag/v0.0.4-original-pmtiles-hotfix.15): owner confirms physical GUI launch, both export buttons and both generated 3MF variants slicing in **both PrusaSlicer and Bambu Studio**. [Evidence](https://github.com/DrFlGd/Map2Model-Next/pull/23#issuecomment-6112594851). The fixture is originally generated **synthetic** 512px WebP/PMTiles, not Interlaken.
- Agent 2 physically offline real Interlaken file **format-only** inspection passed earlier; archive is private. Do not ask owner to repeat it.
- Agent 2 already implemented a *native-only* candidate-preview route on its draft branch: [ELT policy](https://github.com/DrFlGd/Map2Model-Next/blob/agent/2-A2-02-terrarium-grid/crates/data/EXPERIMENTAL-LOCAL-TERRAIN-POLICY.md), [native API and DTO handoff](https://github.com/DrFlGd/Map2Model-Next/blob/agent/2-A2-02-terrarium-grid/crates/data/EXPERIMENTAL-LOCAL-TERRAIN-HANDOFF.md). The separate ExperimentalLocalTerrainPreviewV1 returns candidate heights and unknown flags, **not** a verified ElevationGridV1.
- Agent 3's [draft 3MF exporter PR #15](https://github.com/DrFlGd/Map2Model-Next/pull/15) has already passed real 3MF tests and intentionally refuses unknown-source verified-manifest exports. Preserve that verified path as-is.
- Lead [draft combined prototype PR #23](https://github.com/DrFlGd/Map2Model-Next/pull/23) remains synthetic-only. Do not call it a real-source local experimental 3MF feature.

## Agent 2 review — own crates/data; respond on PR #16 and issue #4

1. **Read and review the new Lead policy**, compare with your existing experimental-native work and submit **APPROVE** or **CHANGES REQUESTED**. Do not start over or create another decoder.
2. Confirm the stable native shape: **decode_experimental_local_pmtiles(local_file, exact_sha256, SelectionV1, zoom, per-job_acknowledgement) → ExperimentalLocalTerrainPreviewV1**. State any additional separately versioned *experimental-only* type fields needed by Lead/Agent 3. No frozen six-contract edits.
3. Confirm fail-closed local-file and consent requirements: bounded regular file, exact SHA/mutation check, actual indexed PMTiles 512px WebP candidate decode, closed seam/coverage, alpha and missing-data failures, contradictory encoding rejection, resource caps, source terms known to prohibit contemplated use, safe untrusted attribution rendering. No network acquisition or automatic downloads; unknown rights/datum remain unknown.
4. Report exact current commit, Windows and Ubuntu CI, negative tests and external-data caveats. Release source bytes and real-derived models must remain **off GitHub**.
5. Separate **private candidate-raster readiness** from **verified production source evidence**; do not treat a checkbox as licensed/certified source or promote preview DTO to verified ElevationGridV1. No self-merge of #14/#16.

## Agent 3 review — own crates/engine and crates/export; respond on PR #15 and issue #6

1. Review how to consume Agent 2's typed **experimental unverified** candidate heights for *printable local evaluation* without forging the trusted ElevationGridV1 or TerrainSourceManifestV1 and without weakening the verified exporter. Propose a **separate experimental mesh/export adapter** and API, reusing genuine mesh QA, 3MF ZIP/OPC, deterministic hashes, atomic/no-clobber and cancellation implementation.
2. Approve or request changes to the **user disclaimer and embedded 3MF provenance**. Preserve experimentalOnly, source SHA, candidate sample SHA, source text attribution/URL, selected bbox/zoom, unknown vertical datum/license/native resolution, redistributionApproved=false, acknowledgement/job identity, uncertainty and warning text in the actual exported 3MF metadata or sidecar. No fake full source approval, no survey-grade heights.
3. Enumerate failure and negative tests: missing consent, unsupported file/encoding, missing tiles or nodata, invalid mesh/hash, cancelled write, aborted worker, unsafe destination and after-publication SHA mismatch. A successfully sliceable 3MF is not evidence of dataset rights/datum.
4. Give Lead minimal API/error-code/job cancellation contracts and actual candidate 3MF independent-reader tests using **original synthetic PMTiles**. Keep frozen contracts unchanged; no automatic third-party data distribution or unrelated exporter rewrite.
5. Confirm Rust dependency NOTICE + full Electron/Node shipping obligations separately from end-user source rights, and answer **APPROVE** or **CHANGES REQUESTED** on PR #15. No self-merge.

## Lead implementation after both responses — issues #8/#9 and PR #23

- Add trusted local-file picker, explicit unchecked **per-job** acknowledgement tied to source SHA + validated selection + job, attribution/unknown-datum warning, safe output Save dialog and persistent **EXPERIMENTAL — PRIVATE LOCAL USE — UNVERIFIED** badges in the UI, project and output.
- Implement stateful native progress/cancellation and true terminal failure checks. No arbitrary renderer file paths, silent network calls, unbounded messages, generated real files in CI, mistaken Generation Ready flags or falsely verified manifest.
- Connect Agent 2's unverified DTO through Agent 3-approved separately typed mesh/3MF bridge. Preserve standard verified source/export behavior, no frozen schema mutations, and record the source disclaimer in actual 3MF provenance.
- Verify original synthetic fixtures, denied/unacknowledged paths, source mutation, cancellation/no-clobber, invalid nodata, exported local metadata and independent 3MF parsing on Windows/Ubuntu before a new prerelease.
- Only then ask owner for **new** physical Windows personal-local-file 3MF test (both slicers and visible warnings). The already passed v0.0.4 synthetic tests need no repetition.

## Review response format

Both agents: comment **APPROVE** or **CHANGES REQUESTED**, exact branch/head + CI, API type signatures, failure/consent rules, local/private-source provenance model, test additions, division of ownership and unresolved legal/vertical-reference facts, in current PR threads (#16 or #15) and assigned issues (#4 or #6). Lead owns integration and subsequent merge decisions. These comments are **design review**, not approval to publish real datasets or promote experimental output into verified mode.
