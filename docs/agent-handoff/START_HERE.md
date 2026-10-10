# Map2Model Next — Multi-Agent Handoff Packet

**Document:** M2M-REQ-001, v1.1, dated 10 October 2026  
**Scope:** Phase 0 foundation + Phase 1 local-first terrain-to-3MF vertical slice  
**Status:** Proposed execution plan; not an approved license assessment or completed implementation.

## Distribution

1. **Product owner and lead:** Read `Map2Model_Local_First_Requirements_v1.1.docx` or `.md` (same product plan, different format).
2. **Agent 1:** Also receive `agent-1-lead-architect.md`. Own foundation, integration, release acceptance.
3. **Agent 2:** Also receive `agent-2-geodata-engine.md`. Review the frozen contracts first; own terrain import/decoder.
4. **Agent 3:** Also receive `agent-3-mesh-and-3mf.md`. Review the frozen contracts first; own geometry/3MF.

## First action — not parallelized

Agent 1 opens and completes **A1-01** (repository, licensing decisions, toolchains, CI and architecture) and **A1-02** (versioned interface contracts, synthetic fixtures and CLI/IPC harness). Agents 2 and 3 review the contracts. The three agents record G0 acceptance and the exact contract commit/hash **before** starting other coding tasks.

## Then parallelize

- Lead works on A1-03 through A1-05: Windows shell, selection and UI, integration, offline acceptance test.
- Geodata agent works on A2-01 and A2-02: PMTiles terrain ingestion and deterministic elevation grids.
- Geometry agent works on A3-01 and A3-02: synthetic terrain mesh, watertight base, 3MF validator and exporter.

All PRs use documented ownership and are integrated by Agent 1. Interfaces may not be silently changed after G0.

## Exit condition

All **M1-AT-01 through M1-AT-08** checks in §21.6 of the requirements must pass, including an actual offline Windows run and 3MF inspection in a reference slicer. The first milestone is **terrain-only**. Overture buildings/roads, browse-as-cache, broader region downloads, search and planets are later roadmap work.

## Integrity of references

The current Map2Model v0.2.0 app and user-supplied 3MF provide behavior/fixture references. Do not copy unlicensed Mini Skyline site code or assume its source assets are redistributable.