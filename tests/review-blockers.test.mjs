import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {selectionDigest,manifestDigest,gridDigest,canonicalMeshDigest} from '../packages/contracts/src/canonical.mjs';
import {projectWgs84,unprojectWgs84,validateSelection,validateManifest,validateGrid,validateBuild,validateMeshResult,validateResultAgainstRequest,validateJobState,validJobTransition} from '../packages/contracts/src/validation.mjs';

const read=(name)=>JSON.parse(readFileSync('tests/fixtures/contracts/v1/'+name+'.json','utf8'));
const selection=read('SelectionV1.valid'),manifest=read('TerrainSourceManifestV1.valid'),
 grid=read('ElevationGridV1.valid'),missing=read('ElevationGridV1.missing'),
 build=read('TerrainBuildRequestV1.valid'),result=read('MeshAnd3mfResultV1.valid'),
 job=read('JobStateV1.valid');
const box=JSON.parse(readFileSync('tests/fixtures/contracts/v1/MeshCanonicalV1.box.json','utf8'));

test('computed canonical selection, manifest, and elevation SHA-256 vectors match independent reference',()=>{
 assert.equal(selectionDigest(selection),'9c13f5eaa44b01562cc6c26796669d44e25bd9ed593420861ef353065c9ae274');
 assert.equal('sel-'+selectionDigest(selection).slice(0,32),selection.selectionId);
 assert.equal(manifestDigest(manifest),'08f6fe9aa91e6415b162fc7b7f42616c302da72bf5fb946a03620c6627e121f3');
 assert.equal(manifest.manifestSha256,manifestDigest(manifest));
 assert.equal(gridDigest(grid),'62ba45025ea9f6f4de60edcfa05136de7097c0a87ce77541322cb1a8bb83b7dd');
 assert.equal(gridDigest(missing),'8f9b1b045fa0f5c1d31da6f927a1da48c4e1e70d565763ecb37db5771aaff833');
 assert.notEqual(manifest.sourceArchiveSha256,manifest.manifestSha256);
 assert.equal(grid.sourceManifestSha256,manifest.manifestSha256);
});
test('computed canonical mesh SHA-256 ignores triangle ordering and vertex permutation',()=>{
 assert.equal(canonicalMeshDigest(box),box.canonicalGeometrySha256);
 const shuffled={vertices:[...box.vertices].reverse(),triangles:box.triangles.map(t=>({partId:t.partId,indices:t.indices.map(i=>box.vertices.length-1-i)})).reverse()};
 assert.equal(canonicalMeshDigest(shuffled),box.canonicalGeometrySha256);
 assert.notEqual(canonicalMeshDigest({...box,vertices:box.vertices.map(v=>v.map(x=>x*1.01))}),box.canonicalGeometrySha256);
});
test('WGS84 linearized EN transform has a known 1 m-ish reference and exact inverse',()=>{
 const anchor=[-84.09,39.741];
 const [east,north]=projectWgs84(anchor,[-84.089,39.742]);
 assert(Math.abs(east-85.71560839369)<1e-6);
 assert(Math.abs(north-111.02965952284)<1e-6);
 const inv=unprojectWgs84(anchor,[east,north]);
 assert(Math.abs(inv[0]+84.089)<1e-12);
 assert(Math.abs(inv[1]-39.742)<1e-12);
 assert.throws(()=>projectWgs84([0,81],[0,81]));
});
test('selection, geodata manifest and grid validators enforce no-data and size limits',()=>{
 assert.equal(validateSelection(selection).ok,true);
 assert.equal(validateManifest(manifest).indexDeclared,false); // Envelope is never proof of tile coverage.
 assert.equal(validateGrid(grid).ok,true);
 assert.equal(validateGrid(missing).missingSamples,1);
 assert.equal(validateBuild({...build,gridSha256:missing.gridSha256},missing).code,'DATA_MISSING');
 assert.equal(validateBuild(build,grid).ok,true);
 assert.equal(validateGrid({...grid,gridSha256:'0'.repeat(64)}).code,'INVALID_GRID_HASH');
 assert.equal(validateGrid({...grid,width:1}).code,'INCOMPLETE_GRID');
 assert.equal(validateGrid({...grid,width:4096,height:4096}).code,'OUT_OF_MEMORY');
 assert.equal(validateGrid({...grid,validMask:grid.validMask.slice(1)}).code,'INCOMPLETE_GRID');
 assert.equal(validateGrid({...grid,validMask:[false,...grid.validMask.slice(1)]}).code,'INCOMPLETE_GRID');
 assert.equal(validateManifest({...manifest,coverageStatus:'verified-tile-index'}).code,'DATA_MISSING');
 assert.equal(validateManifest({...manifest,verticalDatumStatus:'unknown'}).code,'UNSUPPORTED_DATUM');
 assert.equal(validateSelection({...selection,bbox:[10,10,9,11]}).code,'INVALID_SELECTION');
 assert.equal(validateSelection({...selection,bbox:[179,0,-179,1]}).code,'INVALID_SELECTION');
});
test('printing is aspect-preserving and defined for negative terrain height',()=>{
 const v=validateBuild(build,grid);
 assert.equal(v.ok,true);
 assert.equal(v.xyMmPerMeter,5);
 assert.equal(v.zMmForElevation(-8),4);
 assert.equal(v.zMmForElevation(-5),34);
 assert.equal(v.zMmForElevation(12),204);
 assert.equal(validateBuild({...build,modelHeightMm:90},grid).code,'INVALID_DIMENSIONS');
 assert.equal(validateBuild({...build,elevationZeroMeters:0},grid).code,'INVALID_DIMENSIONS');
 assert.equal(validateBuild({...build,clipRectMeters:[0.1,0,36,16]},grid).code,'INVALID_DIMENSIONS');
});
test('3MF result and job contracts reject false success and invalid geometry',()=>{
 assert.equal(validateMeshResult(result).ok,true);
 assert.equal(validateResultAgainstRequest(result,build,grid).ok,true);
 assert.equal(validateResultAgainstRequest({...result,jobId:'different'},build,grid).code,'HASH_MISMATCH');
 assert.equal(validateResultAgainstRequest({...result,artifact:{...result.artifact,exportToken:'m2m-export:other'}},build).code,'HASH_MISMATCH');
 assert.equal(validateResultAgainstRequest({...result,geometry:{...result.geometry,measuredAabbMm:[0,0,0,10,20,30]}},build).code,'INVALID_DIMENSIONS');
 assert.equal(validateResultAgainstRequest({...result,geometry:{...result.geometry,minimumMeasuredThicknessMm:1}},build).code,'INVALID_GEOMETRY');
 assert.equal(validateBuild({...build,selectionId:'sel-impostor'},grid).code,'HASH_MISMATCH');
 assert.equal(validateMeshResult({...result,geometry:{...result.geometry,oriented:false}}).code,'INVALID_GEOMETRY');
 assert.equal(validateMeshResult({...result,artifact:{...result.artifact,conformance:'failed'}}).code,'INVALID_GEOMETRY');
 assert.equal(validateJobState(job).ok,true);
 assert.equal(validateJobState({...job,status:'succeeded',phase:'complete',progress01:0.8}).code,'INTERNAL_ERROR');
 assert.equal(validateJobState({...job,status:'failed',reasonCode:null}).code,'INTERNAL_ERROR');
 assert.equal(validateJobState({...job,status:'cancelled',phase:'cancel',reasonCode:null}).code,'INTERNAL_ERROR');
 assert(validJobTransition(job,{...job,status:'running',phase:'decode',progress01:.4}));
 assert(!validJobTransition({...job,status:'succeeded',phase:'complete',progress01:1},{...job,status:'running',progress01:.5}));
});
