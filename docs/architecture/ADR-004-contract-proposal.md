# ADR-004 — V1 cross-agent data contracts (PROPOSED)

**Task:** A1-02 / G0. **Status:** PROPOSED — NOT FROZEN. **Approval required:** Agent 2 and Agent 3; product owner G0 gate.
**Normative source:** `packages/contracts/schema/v1/*.schema.json`. TS and Rust types implement these exact field names.

## Contract set and ownership
- C-01 `SelectionV1` (UI → geodata): non-wrapping WGS84 bbox `[west,south,east,north]`; Earth only for Phase 1.
- C-02 `TerrainSourceManifestV1` (geodata → lead/mesh): source release, licenses, datum, native resolution and local opaque asset token.
- C-03 `ElevationGridV1` (geodata → mesh): `width * height` row-major north-up sample array, `x = minEast + col*dx`, `y = maxNorth - row*dy`; validMask false means elevation null. Elevations in meters relative to declared datum.
- C-04 `TerrainBuildRequestV1` (lead → mesh): grid identity and clip, print-space mm size and feature minimums, vertical reference, deterministic engine version and export token.
- C-05 `MeshAnd3mfResultV1` (mesh → lead): mm geometry QA, 3MF artifact token, checksums, warnings and provenance.
- C-06 `JobStateV1` (all → UI): stable job/correlation IDs, progress 0–1 and structured reason codes.

## Security and identity invariants
- Renderer sends asset/export **tokens**, never arbitrary filesystem paths, HTTPS URLs or process commands.
- SchemaVersion is `1.0.0`; `kind` must match the contract.
- All source files use SHA-256; hex lower case.
- Non-finite numbers are prohibited. Incomplete terrain must not masquerade as valid by interpolating no-data.
- JSON fixtures in `tests/fixtures/contracts/v1/` are *schema-only synthetic examples*. Their strings of repeated hex digits are placeholders, not checksums of any real data, and the apparent mesh and 3MF result is **not evidence that an export exists**.
- Strict JSON schema rejects unknown fields. Semantic constraints (bbox ordering; `width * height` sample/mask count; missing-elevation/null consistency; request bounds; terminal job codes) are checked separately.

## Requested Agent 2 review
Confirm projection transform choice and error tolerance; allowed `verticalDatumId` and `nativeGroundSampleMeters` representation; mask/null no-data policy; approved source token format and dataset manifest coverage.

## Requested Agent 3 review
Confirm mesh/result validation counters, Z baseline and vertical exaggeration meaning, canonical grid and mesh hash recipe, 3MF conformance states, base/min-thickness and cancellation/event shapes.

## OPEN blocking design decisions
1. **Canonical grid hash:** `gridSha256` exists but serialization/binary float precision is **not frozen**. Hash definitions require both engine owners to agree; fixtures currently use placeholders.
2. **Projection:** `horizontalTransformId` and anchor are fields but EPSG/local tangent-plane algorithm, axis orientation tolerances and datum transforms require review.
3. **Archive handles:** `m2m-asset:<sha256>` is a proposed opaque reference, not a URL or a file path; file authorization policy needs agreement.
4. **IPC envelope:** JSON lines with request/event and correlation IDs is proposed, not a complete stable transport. Cancellation and max message size require consensus.

## G0 sign-off
Record Agent 2 GitHub acknowledgement, Agent 3 GitHub acknowledgement, approved contract commit SHA, exact schema and fixture hashes, tests, source/license policy status, and product owner acceptance. Until then contracts are `PROPOSED` and tasks A2-01/A3-01 remain blocked.
