import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
test('this project has its own versioned package', () => {
  const pkg=JSON.parse(readFileSync('package.json','utf8'));
  assert.equal(pkg.name,'map2model-next');
  assert.equal(pkg.private,true);
  assert(!existsSync('MiniSkyline-Local-Mirror.zip'));
});
test('all agent charters exist', () => {
  for(const file of ['agent-1-lead-architect.md','agent-2-geodata-engine.md','agent-3-mesh-and-3mf.md'])
    assert(existsSync('docs/agent-handoff/'+file));
});
