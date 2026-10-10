import { existsSync, readFileSync } from 'node:fs';
for (const name of [
  'README.md','AGENTS.md',
  'docs/requirements/Map2Model_Local_First_Requirements_v1.1.md',
  'docs/agent-handoff/agent-2-geodata-engine.md',
  'docs/agent-handoff/agent-3-mesh-and-3mf.md',
  'docs/architecture/ADR-003-g0-technical-baseline.md',
  'docs/licensing/SOURCE_POLICY.md',
  'rust-toolchain.toml','Cargo.toml','crates/engine-cli/src/main.rs'
]) {
  if (!existsSync(name)) throw new Error('Missing foundation file: ' + name);
}
const spec = readFileSync('docs/requirements/Map2Model_Local_First_Requirements_v1.1.md','utf8');
for (const id of ['SelectionV1','TerrainSourceManifestV1','ElevationGridV1',
  'TerrainBuildRequestV1','MeshAnd3mfResultV1','JobStateV1']) {
  if (!spec.includes(id)) throw new Error('Requirements omit contract: ' + id);
}
console.log('G0 foundation structure verified');
