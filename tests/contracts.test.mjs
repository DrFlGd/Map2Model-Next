import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import {validateSelection,validateManifest,validateGrid,validateBuild,validateMeshResult,validateJobState} from '../packages/contracts/src/validation.mjs';

const types=['SelectionV1','TerrainSourceManifestV1','ElevationGridV1',
  'TerrainBuildRequestV1','MeshAnd3mfResultV1','JobStateV1'];
const parse=file=>JSON.parse(readFileSync(file,'utf8'));
const fixture=name=>parse('tests/fixtures/contracts/v1/'+name+'.valid.json');
const ajv=new Ajv2020({allErrors:true,strict:true,validateFormats:false});
const validators=new Map(types.map(name=>{
  const schema=parse('packages/contracts/schema/v1/'+name+'.schema.json');
  assert.equal(schema.title,name);
  return [name,ajv.compile(schema)];
}));
function semanticCheck(item){
 const validatorsByKind={
  SelectionV1:validateSelection,TerrainSourceManifestV1:validateManifest,
  ElevationGridV1:validateGrid,TerrainBuildRequestV1:validateBuild,
  MeshAnd3mfResultV1:validateMeshResult,JobStateV1:validateJobState
 };
 return validatorsByKind[item.kind]?.(item).ok===true;
}

test('each canonical V1 fixture validates against Draft 2020-12 JSON Schema',()=>{
  for (const name of types) {
    const data=fixture(name);
    const valid=validators.get(name)(data);
    assert(valid, name+' '+JSON.stringify(validators.get(name).errors));
    assert(semanticCheck(data),name+' semantic check failed');
  }
});
test('all schemas reject unknown fields, version mismatches and missing required fields',()=>{
  for (const name of types) {
    const value=fixture(name);
    assert.equal(validators.get(name)({...value,unexpectedField:1}),false,name);
    assert.equal(validators.get(name)({...value,schemaVersion:'2.0.0'}),false,name);
    const {kind,...withoutKind}=value;
    assert.equal(validators.get(name)(withoutKind),false,name);
  }
});
test('semantic checks reject swapped bounds, missing data, and false readiness',()=>{
  const sel=fixture('SelectionV1');
  assert(!semanticCheck({...sel,bbox:[sel.bbox[2],sel.bbox[1],sel.bbox[0],sel.bbox[3]]}));
  const grid=fixture('ElevationGridV1');
  assert(!semanticCheck({...grid,validMask:grid.validMask.slice(1)}));
  assert(!semanticCheck({...grid,samplesMeters:[null,...grid.samplesMeters.slice(1)]}));
  const job=fixture('JobStateV1');
  assert(!semanticCheck({...job,status:'succeeded',progress01:.5}));
});
test('synthetic grid convention is north-up in meters',()=>{
  const grid=fixture('ElevationGridV1');
  const point=(row,col)=>({
    east:grid.minEastMeters+col*grid.dxMeters,
    north:grid.maxNorthMeters-row*grid.dyMeters,
    z:grid.samplesMeters[row*grid.width+col]
  });
  assert.deepEqual(point(0,0),{east:0,north:16,z:-5});
  assert.deepEqual(point(1,1),{east:12,north:8,z:0});
  assert.deepEqual(point(2,3),{east:36,north:0,z:12});
});
