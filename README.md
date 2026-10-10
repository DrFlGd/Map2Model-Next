# Map2Model Next

**Local-first geographic-to-3D-print desktop application — ground-up redesign**

This repository is for the independent **Map2Model Next** project. The existing Map2Model / Mini Skyline local mirror is **reference material**, not the application's source code or a dependency.

## Project documents

- [M2M-REQ-001 — Requirements and Technical Architecture, v1.1](docs/requirements/Map2Model_Local_First_Requirements_v1.1.md) — authoritative specification, including the multi-agent execution plan (§21)
- [Agent handoff: START HERE](docs/agent-handoff/START_HERE.md) — team sequencing and acceptance gates
- [Agent 1 — Lead Architect & Integrator](docs/agent-handoff/agent-1-lead-architect.md)
- [Agent 2 — Geographic Data Engine](docs/agent-handoff/agent-2-geodata-engine.md)
- [Agent 3 — Geometry & 3MF Engine](docs/agent-handoff/agent-3-mesh-and-3mf.md)
- [Working rules for coding agents](AGENTS.md)

## Current phase: G0 — foundation, not feature implementation

**G0 must happen before parallel feature work.** The lead owns A1-01 (repository, toolchain, CI, licensing/architecture decisions) and A1-02 (versioned cross-agent contracts, fixtures and CLI/IPC harness). Agents 2 and 3 review and explicitly approve the interfaces before work starts on A2/A3.

The first demonstrable milestone is **import local terrain → select geographic bounds → generate watertight terrain → export valid 3MF → repeat with the network disabled on Windows**. Planetary maps, online search, basemap caching, Overture buildings/roads, and full region downloads are subsequent phases.

### Expected repository shape (proposed, to be ratified at G0)

```text
apps/desktop/                Windows UI and desktop shell (Agent 1)
packages/contracts/          Versioned cross-agent contracts (Agent 1)
packages/ui/                 Shared UI components (Agent 1)
crates/data/                 Offline terrain import/decoding (Agent 2)
crates/engine/               Mesh generation (Agent 3)
crates/export/               3MF writer and validation (Agent 3)
tests/                       Fixtures, integration and offline tests
docs/requirements/           Versioned project specification
docs/agent-handoff/          Role-specific kickoff briefs
```

These paths are ownership guidelines rather than completed directories. No claims of working implementation should be made until corresponding acceptance tests run.

## Collaboration rules

- Lead-reviewed PRs, with explicit requirement IDs, testing evidence, fixture hashes, and license/source notes.
- Freeze the six `V1` contracts named in §21.3 at G0, and record the approving commit SHA.
- Respect source licenses, privacy, no-scraping policies, vertical datum and native resolution accuracy. Do not copy unlicensed Mini Skyline implementation code.
- Agent roles are assignments, not autonomous GitHub or background agents; project work begins when contributors are assigned and authorized.

See the [agent handoff guide](docs/agent-handoff/START_HERE.md) to begin.

## Status

- [x] Requirements and agent handoff available
- [ ] G0 architecture and versioned contracts approved
- [ ] Offline terrain vertical slice implemented
- [ ] Windows executable and offline acceptance gate passed
