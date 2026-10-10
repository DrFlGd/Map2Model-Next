# Contributing

Start with [AGENTS.md](AGENTS.md), [project requirements](docs/requirements/Map2Model_Local_First_Requirements_v1.1.md) and [handoff](docs/agent-handoff/START_HERE.md).

## G0 setup
Install Node 22.16.0 and Rust 1.90.0 with rustfmt and clippy. Run:
- `npm run ci`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run --bin map2model-engine -- --health` (stub only)

## Collaboration
The lead owns G0 and reviews all merges. Agents 2 and 3 must accept the V1 contract proposal before independent feature implementations. Link issues, tests, data/fixture hashes, license notes and blockers from every PR. Do not copy upstream website minified bundles.
