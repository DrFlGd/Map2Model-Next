# Map2Model Next

**Independent local-first geographic-to-3D-print desktop application — clean-room redesign**

## Milestones

- **G0 approved / merged:** Six versioned cross-agent contracts at reviewed commit `cabfadb4917fff19ef07325f23ce0fdde30341e3`. [PR #11](https://github.com/DrFlGd/Map2Model-Next/pull/11) records the exact approvals, six schema Git blob IDs, reference fixtures and accepted limits.
- **G1 in development:** Lead (A1-03) implements the Windows desktop shell; Agent 2 (A2-01/A2-02) implements lawful offline Mapterhorn terrain import/decoding; Agent 3 (A3-01/A3-02) implements watertight terrain mesh and genuine 3MF export.
- **G2/G3 not complete:** Integration, offline data readiness, Windows runtime without connectivity and slicer-validation are required before a usable printer export can be claimed.

## Where to begin
- [A1-03 desktop shell](docs/desktop/README.md)
- [Requirements v1.1](docs/requirements/Map2Model_Local_First_Requirements_v1.1.md)
- [Start here — agent assignments](docs/agent-handoff/START_HERE.md)
- [Agent working rules](AGENTS.md)
- [Frozen contracts (schema V1)](packages/contracts/schema/v1)
- [Architecture, security, provenance and licensing](docs/architecture/ADR-004-contract-proposal.md)
- [GitHub issues / work tracking](https://github.com/DrFlGd/Map2Model-Next/issues)

The existing Map2Model/Mini Skyline mirrored app is **reference material only**, not a dependency or source-code transplant.

### Local build
```sh
npm install
npm run ci
cargo test --workspace
npm run desktop:electron
```
Run on Windows to build a portable executable: `npm run desktop:package`.

**Important:** The current A1-03 desktop UI offers an offline coordinate grid, rectangular `SelectionV1` selections and local project save/open. It does not yet render terrain/buildings or export a 3MF; those controls are disabled until Agents 2/3 deliver and G2 integration passes.
