import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import Ajv2020 from 'ajv/dist/2020.js';
import {decodeGridBinary,encodeGridBinary,hydrateGridFromVerifiedRegistry} from '../packages/contracts/src/bulk-grid.mjs';
import {validateGrid,validateBuild,validateSelection,validateManifest,projectWgs84,unprojectWgs84} from '../packages/contracts/src/validation.mjs';
import {selectionDigest,manifestDigest} from '../packages/contracts/src/canonical.mjs';

const f=name=>JSON.parse(readFileSync('tests/fixtures/contracts/v1/'+name+'.json','utf8'));
const selection=f('SelectionV1.valid'),manifest=f('TerrainSourceManifestV1.valid'),grid=f('ElevationGridV1.valid'),
      token=f('ElevationGridV1.token'),build=f('TerrainBuildRequestV1.valid');
const binary=readFileSync('tests/fixtures/contracts/v1/ElevationGridV1.valid.bin');
const bytesSha=x=>createHash('sha256').update(x).digest('hex');
test('contract token-only wire JSON is schema-valid but deliberately never geometry ready',()=>{
 const schema=JSON.parse(readFileSync('packages/contracts/schema/v1/ElevationGridV1.schema.json','utf8'));
 const validate=new Ajv2020({strict:true}).compile(schema);
 assert.equal(validate(token),true,JSON.stringify(validate.errors));
 assert.equal(validateGrid(token).code,'DATA_MISSING');
 assert.equal(validateBuild(build,token).code,'DATA_MISSING');
});
test('G0 signed binary fixture hashes and round-trips identically',()=>{
 assert.equal(bytesSha(binary),'f5e61b6c143b3a3d7027eae439b37f4450f53eeaa8ec853de157486d4b69ae4e');
 assert.deepEqual(encodeGridBinary(grid),binary);
 const decoded=decodeGridBinary(binary);
 assert.deepEqual(decoded.samplesMeters,grid.samplesMeters);
 assert.deepEqual(decoded.validMask,grid.validMask);
});
test('only authorized immutable token bytes can hydrate a grid',()=>{
 const hydrated=hydrateGridFromVerifiedRegistry(token,t=>t===token.gridDataToken?binary:null);
 assert.equal(hydrated.ok,true,JSON.stringify(hydrated));
 assert.equal(hydrated.grid.gridSha256,grid.gridSha256);
 assert.equal(validateBuild(build,hydrated.grid).ok,true);
 assert.equal(hydrateGridFromVerifiedRegistry(token,()=>null).code,'DATA_MISSING');
 const corrupted=Buffer.from(binary);corrupted[corrupted.length-1]^=1;
 assert.equal(hydrateGridFromVerifiedRegistry(token,()=>corrupted).code,'HASH_MISMATCH');
 assert.equal(hydrateGridFromVerifiedRegistry(token,()=>Buffer.concat([binary,Buffer.from([0])])).code,'HASH_MISMATCH');
 assert.equal(hydrateGridFromVerifiedRegistry(token,()=>binary.subarray(0,binary.length-1)).code,'HASH_MISMATCH');
 const bogus={...token,gridDataToken:'m2m-grid:'+'0'.repeat(64)};
 assert.equal(hydrateGridFromVerifiedRegistry(bogus,()=>binary).code,'INVALID_SOURCE');
 assert.throws(()=>decodeGridBinary(binary.subarray(0,11)),/INVALID_SOURCE|INCOMPLETE_GRID/);
 const badFlag=Buffer.from(binary);badFlag[27]=2;
 assert.throws(()=>decodeGridBinary(badFlag),/INVALID_SOURCE/);
});
test('shared ingress rejects unsupported schema fields, no-data repairs and malformed CRS',()=>{
 const s={...selection,terrainProfile:{...selection.terrainProfile,noDataPolicy:'fill'}};
 s.selectionId='sel-'+selectionDigest(s).slice(0,32);
 assert.equal(validateSelection(s).code,'INVALID_SELECTION');
 assert.equal(validateSelection({...selection,planet:'mars'}).code,'INVALID_SELECTION');
 assert.equal(validateSelection({...selection,horizontalCrs:'EPSG:3857'}).code,'INVALID_SELECTION');
 assert.equal(validateSelection({...selection,schemaVersion:'2.0.0'}).code,'INVALID_SELECTION');
 assert.equal(validateSelection({...selection,terrainProfile:{...selection.terrainProfile,targetGroundSampleMeters:Infinity}}).code,'INVALID_SELECTION');
 assert.equal(validateGrid({...grid,anchorWgs84:[181,0]}).code,'INVALID_CRS');
 assert.equal(validateGrid({...grid,anchorWgs84:[0,61]}).code,'INVALID_CRS');
 const declared={...manifest,coverageStatus:'verified-tile-index',coverageIndexSha256:'f'.repeat(64)};
 declared.manifestSha256=manifestDigest(declared);
 const inspected=validateManifest(declared);
 assert.equal(inspected.ok,true);
 assert.equal(inspected.indexDeclared,true);
 assert.equal(Object.hasOwn(inspected,'coverageReady'),false);
});
test('local 2-km WGS84 transform checked against independent WGS84 geodesic vectors',()=>{
 // Reference distances from GeographicLib WGS84.Inverse(2026): see ADR-004.
 const anchor=[0,59.98];
 const east=2000,north=2000;
 const corner=unprojectWgs84(anchor,[east,north]);
 assert(Math.abs(corner[0]-0.03582067436495238)<1e-11);
 assert(Math.abs(corner[1]-59.99795139610392)<1e-11);
 assert.deepEqual(projectWgs84(anchor,corner).map(v=>Math.round(v)),[2000,2000]);
 const northEdgeGeodesic=1998.9172358132557;
 const westEdgeGeodesic=2000.0027391521326;
 assert(Math.abs(northEdgeGeodesic-east)<=2); // < 0.1% for each supported side.
 assert(Math.abs(westEdgeGeodesic-north)<=2);
 const overLat={...selection,bbox:[0,60,0.01,60.00001]};
 assert.equal(validateSelection(overLat).code,'INVALID_SELECTION');
 const overSize={...selection,bbox:[0,0,0.04,0.01]};
 assert.equal(validateSelection(overSize).code,'INVALID_SELECTION');
});
