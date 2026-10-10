# ADR-003 — G0 technical baseline

**Status:** Accepted as a *scaffold*, not final contract ratification.
**Owner:** Agent 1. **Linked task:** A1-01. **Date:** 2026-10-10.
**Source:** M2M-REQ-001 v1.1 §§11–15,21.

## Proposed architecture

- Independent clean-room implementation. The prior mirrored website is a behavior and provenance reference only.
- Windows target. Electron and React/MapLibre planned as a sandboxed UI with a separately launched crash-isolated Rust engine.
- Native code owns local filesystem, imported geodata and model generation. The renderer receives only narrow validated IPC methods.
- Engine wire messages will be versioned UTF-8 JSON with operation, correlation ID, typed error and progress events. Limit message sizes and resolve user-approved file tokens in the native side.
- Pin Node 22.16.0 and Rust 1.90.0 for the initial skeleton; pin all later dependencies.
- Shared V1 schemas are canonical, with equivalent TypeScript and Rust contracts and JSON fixtures.
- Ownership: Agent 1 desktop/contracts/CI; Agent 2 data decoder/import; Agent 3 mesh/export.
- Phase 1 scope: local, small Earth/WGS84 rectangular terrain to watertight 3MF. Buildings, maps, search and planetary data follow later.

## Open questions for Agents 2 and 3 before G0 sign-off

1. Local East/North metric projection, anchored coordinate origin, vertical datum and numerical tolerance.
2. Row-major mask/elevation representation in CLI JSON and future large-buffer transport.
3. Canonical mesh hash order, floating-point tolerance, 3MF library compatibility and slicer test versions.
4. Permissioned local archive identifiers without accepting arbitrary renderer paths.
5. License terms and fixture rights for sample Mapterhorn terrain packs.

## Evidence and limitations

Initial CI proves only Rust/Node scaffolding. The engine --health stub cannot decode elevation, generate meshes or export 3MF. Offline functionality is a separate G3 acceptance gate.
