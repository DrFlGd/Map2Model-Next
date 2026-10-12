import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {selectionFromBbox} from '../apps/desktop/shared/selection.mjs';
import {coveragePolygon,coverageMessage,coverageAdmitsExperiment,approvedExperimentalExportPayload} from '../apps/desktop/shared/coverage.mjs';

const {registerExperimentalHandlers}=createRequire(import.meta.url)('../apps/desktop/electron/experimental.cjs');
const SOURCE='a'.repeat(64);
const BBOX=[-84.11132,39.63954,-83.93555,39.77476];
const sha=b=>createHash('sha256').update(b).digest('hex');

test('map coverage helpers label header-only boundaries and never promote unknown or partial selection',()=>{
 const geo=coveragePolygon(BBOX);
 assert.equal(geo.features[0].geometry.type,'Polygon');
 assert.deepEqual(geo.features[0].geometry.coordinates[0][0],[-84.11132,39.63954]);
 assert.deepEqual(geo.features[0].geometry.coordinates[0].at(-1),geo.features[0].geometry.coordinates[0][0]);
 assert.deepEqual(coveragePolygon([0,0,0,0]).features,[]);
 assert.match(coverageMessage(null,true,false),/Draw a rectangle/);
 assert.match(coverageMessage({status:'partial',missingRequiredTiles:2},true,true),/Missing required tiles: 2/);
 assert.match(coverageMessage({status:'outside',zoomAvailable:false},true,true),/zoom/);
 const selection=selectionFromBbox([-84.024,39.7067,-84.023,39.7077]);
 const source={jobId:'job-1',sourceArchiveSha256:SOURCE,zoom:12};
 const inside={jobId:'job-1',selectionId:selection.selectionId,sourceArchiveSha256:SOURCE,
   zoom:12,status:'inside',exportEligible:true,missingRequiredTiles:0};
 assert.equal(coverageAdmitsExperiment(inside,selection,source),true);
 assert.equal(coverageAdmitsExperiment({...inside,status:'partial'},selection,source),false);
 assert.equal(coverageAdmitsExperiment({...inside,sourceArchiveSha256:'b'.repeat(64)},selection,source),false);
 assert.equal(coverageAdmitsExperiment({...inside,selectionId:'wrong'},selection,source),false);
});

test('G2 regression: source-first null selectionId must not leak into an inside-approved export payload',async()=>{
 const selection=selectionFromBbox([-84.024,39.7067,-84.023,39.7077]);
 const receipt={jobId:'source-first-123',selectionId:null,sourceArchiveSha256:SOURCE,zoom:12};
 const inside={jobId:receipt.jobId,selectionId:selection.selectionId,
   sourceArchiveSha256:SOURCE,zoom:12,status:'inside',exportEligible:true,
   missingRequiredTiles:0};
 assert.equal(receipt.selectionId,null,'choosing local source precedes drawing selection');
 const payload=approvedExperimentalExportPayload(inside,selection,receipt,true);
 assert.deepEqual(payload,{jobId:receipt.jobId,selectionId:selection.selectionId,accepted:true});
 assert.notEqual(payload.selectionId,receipt.selectionId,
   'BUG REGRESSION: never forward source-first null selectionId to native exporter');
 assert(Object.isFrozen(payload));
 assert.throws(()=>approvedExperimentalExportPayload(inside,selection,receipt,false),/consent/);
 assert.throws(()=>approvedExperimentalExportPayload({...inside,status:'partial'},selection,receipt,true),/indexed coverage/);
 assert.throws(()=>approvedExperimentalExportPayload({...inside,selectionId:'stale'},selection,receipt,true),/indexed coverage/);
 const changedSelection=selectionFromBbox([-84.0239,39.7067,-84.023,39.7077]);
 assert.throws(()=>approvedExperimentalExportPayload(inside,changedSelection,receipt,true),/indexed coverage/);
 // Ensure the real shipped UI calls the same tested shared builder rather
 // than accidentally forwarding the source-first receipt ID again.
 const jsx=await fs.readFile(new URL('../apps/desktop/src/App.jsx',import.meta.url),'utf8');
 assert.match(jsx,/const request=approvedExperimentalExportPayload\(/);
 assert.match(jsx,/api\.exportExperimentalLocal\(\s*request\.jobId,request\.selectionId,request\.accepted\)/);
 assert(!jsx.includes('api.exportExperimentalLocal(job.jobId,job.selectionId,true)'));
});

test('experiment requires source-first picker, current SHA-bound indexed coverage and one-use consent',async()=>{
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'m2m-coverage-'));
 try{
   const sourceFile=path.join(root,'original.pmtiles');
   const out=path.join(root,'model.3mf');
   await fs.writeFile(sourceFile,Buffer.alloc(128,65));
   const selection=selectionFromBbox([-84.024,39.7067,-84.023,39.7077]);
   const selectionOther=selectionFromBbox([-84.032,39.707,-84.031,39.708]);
   const handlers=new Map(),calls=[];
   let saves=0,condition='inside';
   const bytes=Buffer.from('only original test synthetic sample in fake native result');
   registerExperimentalHandlers({
     handler:(name,fn)=>handlers.set(name,fn),
     app:{getPath:()=>root},getWindow:()=>null,nativeEnginePath:()=>'/packaged/native.exe',
     dialog:{
       showOpenDialog:async()=>({canceled:false,filePaths:[sourceFile]}),
       showSaveDialog:async()=>{saves++;return {canceled:false,filePath:out};}
     },
     runNative:async(_exe,args)=>{
       calls.push(args);
       if(args[0]==='--inspect-local-experimental')
         return {stdout:JSON.stringify({status:'ok',result:{
           inspectionKind:'UnverifiedLocalPMTilesFormatV1',archiveBytes:128,
           archiveSha256:SOURCE,headerZoomRangeConsistent:true,actualIndexedMaxZoom:12,
           sourceLicenseVerified:false,productionGenerationReady:false,
           sourceAttributionFromArchive:'Test & <untrusted>'
         }})};
       if(args[0]==='--coverage-local'){
         if(args.length===2)
           return {stdout:JSON.stringify({status:'ok',result:{
             kind:'LocalPMTilesCoveragePreviewV1',sourceArchiveSha256:SOURCE,sourceArchiveBytes:128,
             productionGenerationReady:false,sourceLicenseVerified:false,
             verticalDatumStatus:'unknown',networkRequests:0,
             headerMinZoom:12,headerMaxZoom:12,indexedTileCount:4,
             declaredCoverageBboxWgs84:BBOX,
             coverageGeoJson:coveragePolygon(BBOX).features[0],
             boundarySource:'PMTiles header approximate'
           }})};
         assert.equal(args[2],SOURCE,'selection check pins original archive digest');
         assert.equal(args[7],'12');
         const query=args.slice(3,7).map(Number);
         return {stdout:JSON.stringify({status:'ok',result:{
           kind:'LocalPMTilesCoveragePreviewV1',sourceArchiveSha256:SOURCE,
           productionGenerationReady:false,networkRequests:0,
           selection:{requestedZoom:12,bboxWgs84:query,status:condition,zoomAvailable:true,
             requiredClosedBoundaryTiles:4,indexedRequiredTiles:condition==='inside'?4:2,
             missingRequiredTiles:condition==='inside'?0:2,
             allRequiredTilesIndexed:condition==='inside',
             classificationScope:'index-only; pixels not verified'}
         }})};
       }
       assert.equal(args[0],'--export-local-experimental');
       assert.equal(args[1],sourceFile);
       assert.equal(args[2],SOURCE);
       assert.equal(args[4],'12');
       const nativeSelection=JSON.parse(await fs.readFile(args[3],'utf8'));
       assert.deepEqual(nativeSelection,selection);
       await fs.writeFile(args[7],bytes,{flag:'wx'});
       return {stdout:JSON.stringify({status:'ok',result:{
         experimentalOnly:true,productionGenerationReady:false,redistributionApproved:false,
         threeMfExported:true,jobId:args[5],selectionId:selection.selectionId,
         sourceArchiveSha256:SOURCE,packageSha256:sha(bytes),bytes:bytes.length,
         candidateHeightsSha256:'b'.repeat(64),vertices:100,triangles:196,aabbMm:[0,0,0,120,100,50]
       }})};
     }
   });
   const choose=handlers.get('selectExperimentalSource');
   const assess=handlers.get('assessExperimentalCoverage');
   const exportFn=handlers.get('exportExperimentalLocal');
   assert.equal(typeof choose,'function');
   assert.equal(typeof assess,'function');
   const receipt=await choose({});
   assert.equal(receipt.selectionId,null,'source chosen BEFORE any rectangle');
   assert.deepEqual(receipt.declaredCoverageBboxWgs84,BBOX);
   assert(!JSON.stringify(receipt).includes(sourceFile),'native absolute source path stays private');
   assert.equal(receipt.sourceAttributionFromArchive,'Test & <untrusted>');
   await assert.rejects(exportFn({jobId:receipt.jobId,selectionId:selection.selectionId,accepted:false}),/consent/);
   assert.equal(saves,0);
   condition='partial';
   const partial=await assess({jobId:receipt.jobId,selection});
   assert.equal(partial.status,'partial');
   assert.equal(partial.exportEligible,false);
   await assert.rejects(exportFn({jobId:receipt.jobId,selectionId:selection.selectionId,accepted:true}),/indexed tile coverage/);
   assert.equal(saves,0);
   const fresh=await choose({});
   condition='inside';
   const verdict=await assess({jobId:fresh.jobId,selection});
   assert.equal(verdict.status,'inside');
   assert.equal(verdict.exportEligible,true);
   assert.equal(verdict.missingRequiredTiles,0);
   await assert.rejects(exportFn({jobId:fresh.jobId,selectionId:selectionOther.selectionId,accepted:true}),/coverage/);
   assert.equal(saves,0);
   const authorized=await choose({});
   assert.equal(authorized.selectionId,null,'source-first receipt has no selection ID');
   const insideAssessment=await assess({jobId:authorized.jobId,selection});
   const authorizedPayload=approvedExperimentalExportPayload(
     insideAssessment,selection,authorized,true);
   const published=await exportFn(authorizedPayload);
   assert.equal(published.packageSha256,sha(bytes));
   assert.deepEqual(await fs.readFile(out),bytes);
   assert.equal(saves,1);
   await assert.rejects(exportFn({jobId:authorized.jobId,selectionId:selection.selectionId,accepted:true}),/expired/);
   const replacement=await choose({});
   await assess({jobId:replacement.jobId,selection});
   await assert.rejects(exportFn({jobId:replacement.jobId,selectionId:selection.selectionId,accepted:true}),/atomic publication/);
   assert.deepEqual(await fs.readFile(out),bytes,'no-clobber preserves prior model');
   assert.equal(saves,2);
   assert.equal(calls.filter(x=>x[0]==='--export-local-experimental').length,2);
 }finally{await fs.rm(root,{recursive:true,force:true});}
});

test('no local source receipt means no export, Save dialog or native worker',async()=>{
 let saves=0,executions=0;
 const operations=new Map();
 registerExperimentalHandlers({
   handler:(name,fn)=>operations.set(name,fn),
   app:{getPath:()=>'/tmp'},getWindow:()=>null,nativeEnginePath:()=>'/native',
   dialog:{showSaveDialog:async()=>{saves++;return {canceled:true};}},
   runNative:async()=>{executions++;throw Error('should not run native');}
 });
 await assert.rejects(operations.get('exportExperimentalLocal')({jobId:'missing',accepted:false}),/consent/);
 await assert.rejects(operations.get('exportExperimentalLocal')({jobId:'missing',accepted:true}),/expired/);
 assert.equal(saves,0);assert.equal(executions,0);
});
