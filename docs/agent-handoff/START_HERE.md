# Map2Model Next — Multi-Agent Handoff Packet

**Document:** M2M-REQ-001, v1.1, dated 10 October 2026  
**Scope:** Phase 0 foundation + Phase 1 local-first terrain-to-3MF vertical slice  
**Status:** G0 approved/merged; A3-01 merged; Windows v0.0.4 synthetic 512px PMTiles-to-3MF and original fixed-demo exports owner-tested successfully in **both PrusaSlicer and Bambu Studio**. Owner has now approved a separate **private/local experimental source** path with explicit licensing/provenance/unknown-datum disclaimer; this is NOT source-rights clearance, real elevation accuracy certification or authorization to distribute data. See current G2 review dispatch below. Original kickoff sequence is historical context.

## Current dispatch — updated October 11, 2026

**Read the current, authoritative instructions before working from the original role briefs:**

- **[CURRENT G2 review — private-use experimental local terrain and 3MF](G2-PRIVATE-USE-EXPERIMENTAL-TERRAIN-REVIEW-2026-10-11.md)** — **READ FIRST**; Agent 2 and Agent 3 review the owner-selected disclaimer, separate experimental DTO/export boundaries, consent and attribution before Lead implementation. [Full policy](../licensing/PRIVATE-LOCAL-TERRAIN-POLICY-2026-10-11.md).
- **[POST-v0.0.2 next actions — AGENT 2, AGENT 3 AND LEAD](POST-V0.0.2-NEXT-ACTIONS-2026-10-11.md)** — **HISTORICAL; superseded on private experimental source gating**; basic Windows synthetic GUI/export/dual-slicer smoke accepted, next native lifecycle, original synthetic PMTiles grid, 3MF notice and Mapterhorn provenance gates.
- **[A1-04 integration dispatch and latest Agent 2/3 acceptance gates](A1-04-INTEGRATION-DISPATCH-2026-10-10.md)** — newest exact PR heads, real-source format vs authorization, actual Rust 3MF slicer testing, and lead implementation sequence. Read this first.
- **[G1 active actions for Agent 2, Agent 3 and lead](G1-NEXT-ACTIONS-2026-10-10.md)** — exact PR links, present acceptance gaps, 3MF writer decision, agent ownership and synthetic-first Windows build sequence.
- **[Real Mapterhorn sample and source-datum protocol](MAPTERHORN-SAMPLE-PROTOCOL.md)** — official endpoints, test extract commands, attribution/licensing diligence, real 512x512 data inspection, and fail-closed vertical datum rules.

Do **not** copy the private legacy Mini Skyline website or claim that its old online cache constitutes a verified source archive. This repository's G0 six V1 contracts are already frozen; do not change them without formal signoff.

## Distribution

1. **Product owner and lead:** Read `Map2Model_Local_First_Requirements_v1.1.docx` or `.md` (same product plan, different format).
2. **Agent 1:** Also receive `agent-1-lead-architect.md`. Own foundation, integration, release acceptance.
3. **Agent 2:** Also receive `agent-2-geodata-engine.md`. Review the frozen contracts first; own terrain import/decoder.
4. **Agent 3:** Also receive `agent-3-mesh-and-3mf.md`. Review the frozen contracts first; own geometry/3MF.

## Historical G0 approval step — already completed

Agent 1 completed **A1-01** and **A1-02** and obtained both agents' contract signoffs and owner acceptance. The frozen G0 contract baseline was merged in commit `4807a5b`. **No agent needs to wait for another G0 approval** before the current G1 tasks described in the current dispatch.

## Current workstreams

- Lead works on A1-03 through A1-05: Windows shell, selection and UI, integration, offline acceptance test.
- Geodata agent works on A2-01 and A2-02: PMTiles terrain ingestion and deterministic elevation grids.
- Geometry agent works on A3-01 and A3-02: synthetic terrain mesh, watertight base, 3MF validator and exporter.

All PRs use documented ownership and are integrated by Agent 1. Interfaces may not be silently changed after G0.

## Exit condition

All **M1-AT-01 through M1-AT-08** checks in §21.6 of the requirements must pass, including an actual offline Windows run and 3MF inspection in a reference slicer. The first milestone is **terrain-only**. Overture buildings/roads, browse-as-cache, broader region downloads, search and planets are later roadmap work.

## Integrity of references

The current Map2Model v0.2.0 app and user-supplied 3MF provide behavior/fixture references. Do not copy unlicensed Mini Skyline site code or assume its source assets are redistributable.
