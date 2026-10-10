# MAP2MODEL NEXT — AGENT KICKOFF BRIEF

Specification: **M2M-REQ-001 v1.1**; milestone: **Phase 0/1**; status: proposed, awaiting G0 contract ratification.

Read the full requirements §1–21 before coding, especially §11 (contracts), §14 (rights/security), §15 (tests), §16 (roadmap), and §21 (agent operating model). Do not copy the mirrored Mini Skyline runtime. Coordinate through versioned contracts and lead-reviewed PRs.

## Agent 3 brief — Geometry Kernel, Mesh Validation, and 3MF

**Mission:** Build the offline printable-terrain engine against the frozen `ElevationGridV1` contract. Start with a synthetic elevation grid so work proceeds independently of Agent 2; generate a clipped relief, watertight sides and base, unit-correct dimensions, controlled Z scaling and print-quality diagnostics. Use a standards-conformant 3MF writer, document the library/license decision, embed source and engine provenance, and verify outputs in selected slicers. **Deliver:** mesh/export crate PRs, synthetic/real-grid fixtures, manifold/3MF validation reports, deterministic canonical mesh hashes and reproducible headless CLI demonstration. **Do not:** couple to UI JavaScript, consume raw PMTiles or undocumented geodata shapes, substitute arbitrary heights, or accept non-manifold geometry as printable.

## Assigned task IDs

A3-01, A3-02

## Primary path ownership

crates/engine/, crates/export/, mesh/3MF fixtures and tests

## Shared contracts and exit gates

- G0 is owned by Agent 1. Agents 2 and 3 review and approve contracts before feature work begins.
- All changes use the frozen V1 contracts from §21.3. Interface changes require the lead's approval and versioned tests.
- Submit a PR with fixture IDs, test commands, evidence, license/provenance notes and linked requirements.
- Initial release exits only when all M1-AT-01 through M1-AT-08 pass on a network-disabled Windows machine.

## Communication

Raise blockers for CRS, vertical datum, source licensing, missing data, determinism, mesh validity or IPC immediately to Agent 1. Do not silently fill gaps, change field types, or substitute live network calls.
