# Map2Model Next — Agent Working Rules

**Authority:** [M2M-REQ-001 v1.1](docs/requirements/Map2Model_Local_First_Requirements_v1.1.md), especially §11 and §21. Read [START HERE](docs/agent-handoff/START_HERE.md) and your role-specific handoff before coding.

## Mandatory G0 gate

1. Agent 1 establishes A1-01 repository/toolchain/CI/licensing decisions, then proposes A1-02 versioned contracts and fixtures.
2. Agents 2 and 3 review and explicitly approve the contracts with the lead.
3. Record the approved contract commit SHA and G0 decision. **No parallel feature development before G0**.

## Contracts (V1)

`SelectionV1`, `TerrainSourceManifestV1`, `ElevationGridV1`, `TerrainBuildRequestV1`, `MeshAnd3mfResultV1`, `JobStateV1`.

Never change frozen fields/types without a lead-approved versioned migration and fixture updates.

## Agent ownership

| Agent | Primary ownership | Brief |
|---|---|---|
| 1 — Lead Architect & Integrator | `apps/desktop/`, `packages/contracts/`, `packages/ui/`, CI, integration | [Brief](docs/agent-handoff/agent-1-lead-architect.md) |
| 2 — Geodata Engine | `crates/data/`, PMTiles and elevation fixtures | [Brief](docs/agent-handoff/agent-2-geodata-engine.md) |
| 3 — Mesh/3MF Engine | `crates/engine/`, `crates/export/`, mesh validation | [Brief](docs/agent-handoff/agent-3-mesh-and-3mf.md) |

Avoid cross-owner code changes without review. Work through PRs linked to A1/A2/A3 task IDs. State what you tested, the fixture/data source and its license, reproducibility notes, and any blocked acceptance checks.

## Scope and correctness

- First milestone: **fully offline Windows terrain-only path**, from local import through verifiable watertight 3MF.
- Terrain elevation: explicit coordinate reference system, units, vertical datum, provenance, and no-data behavior; never equate zoom with native accuracy.
- Mesh: deterministic canonical geometry and 3MF validation; no invented heights or silently accepted non-manifold models.
- Use legally distributable data and original code. Do not copy the proprietary/minified Mini Skyline runtime.
- Do not claim offline-ready until M1-AT-01 through M1-AT-08 pass on a network-disabled Windows machine.
