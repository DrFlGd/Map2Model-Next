# MAP2MODEL NEXT — AGENT KICKOFF BRIEF

Specification: **M2M-REQ-001 v1.1**; milestone: **Phase 0/1**; status: proposed, awaiting G0 contract ratification.

Read the full requirements §1–21 before coding, especially §11 (contracts), §14 (rights/security), §15 (tests), §16 (roadmap), and §21 (agent operating model). Do not copy the mirrored Mini Skyline runtime. Coordinate through versioned contracts and lead-reviewed PRs.

## Agent 2 brief — Geodata Import, Terrain Decoder, and Coverage

**Mission:** Build the independent offline data path for the first Earth terrain slice. Wait for signed G0 contracts, then implement a local licensed Mapterhorn PMTiles import/reader, tile index and integrity checks, elevation decoding, spatial clipping/sampling, WGS84-to-local-metric conversion and explicit vertical datum handling. Produce deterministic `ElevationGridV1` responses and manifest hashes for both synthetic and real terrain fixtures. Own decode/no-data/edge tests and document any mismatch between claimed and actual native source resolution. **Deliver:** data-crate PR, fixtures, sample CLI outputs, source/license records, deterministic grid and error tests. **Do not:** embed download-on-demand into generation, assume zoom is native accuracy, bulk-scrape public tiles, or silently interpolate a missing terrain cell.

## Assigned task IDs

A2-01, A2-02

## Primary path ownership

crates/data/, terrain fixtures, importer/decoder tests

## Shared contracts and exit gates

- G0 is owned by Agent 1. Agents 2 and 3 review and approve contracts before feature work begins.
- All changes use the frozen V1 contracts from §21.3. Interface changes require the lead's approval and versioned tests.
- Submit a PR with fixture IDs, test commands, evidence, license/provenance notes and linked requirements.
- Initial release exits only when all M1-AT-01 through M1-AT-08 pass on a network-disabled Windows machine.

## Communication

Raise blockers for CRS, vertical datum, source licensing, missing data, determinism, mesh validity or IPC immediately to Agent 1. Do not silently fill gaps, change field types, or substitute live network calls.