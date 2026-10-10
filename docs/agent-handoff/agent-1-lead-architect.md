# MAP2MODEL NEXT — AGENT KICKOFF BRIEF

Specification: **M2M-REQ-001 v1.1**; milestone: **Phase 0/1**; status: proposed, awaiting G0 contract ratification.

Read the full requirements §1–21 before coding, especially §11 (contracts), §14 (rights/security), §15 (tests), §16 (roadmap), and §21 (agent operating model). Do not copy the mirrored Mini Skyline runtime. Coordinate through versioned contracts and lead-reviewed PRs.

## Agent 1 brief — Lead Architect, Desktop, and Integration

**Mission:** Create an independent Windows Map2Model Next application and own its end-to-end integration. Start alone at G0. Establish a clean-room repository and lock `SelectionV1`, `TerrainSourceManifestV1`, `ElevationGridV1`, `TerrainBuildRequestV1`, `MeshAnd3mfResultV1`, and `JobStateV1` with schema fixtures and engine-CLI/IPC stubs. After Agents 2 and 3 approve the contracts, build the React/MapLibre/Electron shell, local project persistence, rectangular selection, status/error UX, and safe engine process bridge. Merge approved PRs, create installer artifacts, and run the complete offline test. **Deliver:** G0 contract PR, desktop integration PR, reproducible build instructions, M1-AT-01 through M1-AT-08 evidence, and the consolidated acceptance report. **Do not:** fork or copy the minified Mini Skyline runtime, override GIS/mesh ownership without review, or claim “offline ready” without a no-network test.

## Assigned task IDs

A1-01, A1-02, A1-03, A1-04, A1-05

## Primary path ownership

apps/desktop/, packages/contracts/, packages/ui/, .github/workflows/, integration tests

## Shared contracts and exit gates

- G0 is owned by Agent 1. Agents 2 and 3 review and approve contracts before feature work begins.
- All changes use the frozen V1 contracts from §21.3. Interface changes require the lead's approval and versioned tests.
- Submit a PR with fixture IDs, test commands, evidence, license/provenance notes and linked requirements.
- Initial release exits only when all M1-AT-01 through M1-AT-08 pass on a network-disabled Windows machine.

## Communication

Raise blockers for CRS, vertical datum, source licensing, missing data, determinism, mesh validity or IPC immediately to Agent 1. Do not silently fill gaps, change field types, or substitute live network calls.