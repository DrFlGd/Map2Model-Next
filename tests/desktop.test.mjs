import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm,readdir,readFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {selectionFromBbox,assertSelection} from '../apps/desktop/shared/selection.mjs';
import {selectionDigest} from '../packages/contracts/src/canonical.mjs';
import {validateSelection} from '../packages/contracts/src/validation.mjs';
import {createProject,loadProject,saveProject,listProjects,assertProject} from '../apps/desktop/shared/projects.mjs';

test('SelectionV1 builder uses the frozen contract digest and rejects invalid coordinate selections',()=>{
 const bounds=[-84.09,39.74,-84.087,39.742];
 const selection=selectionFromBbox(bounds);
 assert.equal(selection.kind,'SelectionV1');
 assert.equal(selection.selectionId,'sel-'+selectionDigest(selection).slice(0,32));
 assert.equal(validateSelection(selection).ok,true);
 for(const coords of [[-84.09,39.74,-84.087,39.742],[0,0,0.005,0.005],[7,46.6,7.01,46.61]]){
   const candidate=selectionFromBbox(coords);
   assert.equal(candidate.selectionId,'sel-'+selectionDigest(candidate).slice(0,32));
   assert.equal(validateSelection(candidate).ok,true);
 }
 assert.equal(selection.selectionId.length,36);
 assert.equal(assertSelection(selection).selectionId,selection.selectionId);
 assert.deepEqual(selection.bbox,bounds);
 assert.throws(()=>selectionFromBbox([2,1,0,3]),/INVALID_SELECTION/);
 assert.throws(()=>selectionFromBbox([179,0,-179,1]),/INVALID_SELECTION/);
 assert.throws(()=>selectionFromBbox([0,60.01,0.001,60.02]),/INVALID_SELECTION/);
 assert.throws(()=>selectionFromBbox([-84.09,39.74,-84.01,39.75]),/INVALID_SELECTION/);
 assert.throws(()=>assertSelection({...selection,selectionId:'sel-'+'0'.repeat(32)}),/INVALID_SELECTION/);
 assert.throws(()=>assertSelection({...selection,planet:'mars'}),/INVALID_SELECTION/);
 assert.throws(()=>assertSelection({...selection,bbox:[1,2,3,4]}),/INVALID_SELECTION/);
 assert.throws(()=>assertSelection({...selection,unknownField:1}),/INVALID_SELECTION/);
});

test('Library projects survive close/reopen without network; only approved IDs can be resolved',async()=>{
 const root=await mkdtemp(path.join(os.tmpdir(),'m2m-desktop-'));
 try{
  const created=await createProject(root,'Local landscape');
  const sel=selectionFromBbox([-84.09,39.74,-84.087,39.742]);
  const saved=await saveProject(root,{projectId:created.projectId,title:'Local landscape 2',selection:sel});
  const loaded=await loadProject(root,created.projectId);
  assert.equal(loaded.title,'Local landscape 2');
  assert.deepEqual(loaded.selection,sel);
  assert.equal((await listProjects(root)).length,1);
  assert.equal(saved.createdAt,created.createdAt);
  assert(loaded.updatedAt>=created.updatedAt);
  assert.throws(()=>assertProject({...created,formatVersion:99}),/INVALID_PROJECT/);
  await assert.rejects(loadProject(root,'../../etc/passwd'),/INVALID_PROJECT/);
  await assert.rejects(saveProject(root,{projectId:created.projectId,title:'',selection:sel}),/INVALID_PROJECT/);
  await assert.rejects(saveProject(root,{projectId:created.projectId,title:'Okay',selection:{...sel,planet:'mars'}}),/INVALID_SELECTION/);
  const files=await readdir(path.join(root,'projects'));
  assert.equal(files.length,1);
  const data=await readFile(path.join(root,'projects',files[0]),'utf8');
  assert(!data.includes('apiKey')&&!data.includes('https://'));
 }finally{await rm(root,{recursive:true,force:true});}
});

test('desktop defaults to a bundled local map style and blocks runtime external networks',async()=>{
 const {readFileSync}=await import('node:fs');
 const js=readFileSync('apps/desktop/src/App.jsx','utf8');
 const main=readFileSync('apps/desktop/electron/main.cjs','utf8');
 const preload=readFileSync('apps/desktop/electron/preload.cjs','utf8');
 const html=readFileSync('apps/desktop/index.html','utf8');
 assert(js.includes("sources:{}"));
 assert(js.includes('No external map tiles are requested'));
 assert(!/https?:\/\/(?:tiles\.|api\.|demotiles\.)/.test(js));
 assert(main.includes("networkEnabled:false"));
 assert(main.includes("session.defaultSession"));
 assert(main.includes("webRequest.onBeforeRequest"));
 assert(main.includes("sandbox:true"));
 assert(main.includes("contextIsolation:true"));
 assert(main.includes("nodeIntegration:false"));
 assert(main.includes("setWindowOpenHandler(()=>({action:'deny'})"));
 assert(preload.includes("contextBridge.exposeInMainWorld"));
 assert(html.includes("default-src 'self'"));
 assert(!html.includes('unpkg.com')&&!html.includes('fonts.googleapis.com'));
});
