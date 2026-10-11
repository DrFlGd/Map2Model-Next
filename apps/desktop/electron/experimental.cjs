'use strict';
const fs=require('node:fs/promises');
const path=require('node:path');
const {createHash,randomUUID}=require('node:crypto');
const {execFile}=require('node:child_process');
const {promisify}=require('node:util');
const runBinary=promisify(execFile);
const POLICY='PRIVATE-LOCAL-TERRAIN-2026-10-11';
function registerExperimentalHandlers({handler,app,dialog,getWindow,nativeEnginePath}){
 const pending=new Map();
 handler('selectExperimentalSource',async payload=>{
   const {assertSelection}=await import('../shared/selection.mjs');
   const selection=assertSelection(payload.selection);
   const answer=await dialog.showOpenDialog(getWindow(),{
     title:'Select an already acquired LOCAL PMTiles file',
     filters:[{name:'Local terrain PMTiles',extensions:['pmtiles']}],
     properties:['openFile']});
   if(answer.canceled||!answer.filePaths?.[0])return {canceled:true};
   const sourcePath=path.resolve(answer.filePaths[0]);
   const info=await fs.lstat(sourcePath);
   if(!info.isFile()||info.isSymbolicLink()||info.size<127||info.size>128*1024*1024)
     throw Error('INVALID_SOURCE: expected a bounded, ordinary local PMTiles file');
   let stdout;
   try{
     ({stdout}=await runBinary(nativeEnginePath(),['--inspect-local-experimental',sourcePath],
       {windowsHide:true,timeout:120000,maxBuffer:65536,encoding:'utf8'}));
   }catch{throw Error('INVALID_SOURCE: local PMTiles inspection failed');}
   let checked;
   try{checked=JSON.parse(stdout);}catch{throw Error('INVALID_SOURCE: invalid inspector response');}
   const src=checked.result;
   if(checked.status!=='ok'||src?.inspectionKind!=='UnverifiedLocalPMTilesFormatV1'
     ||src.productionGenerationReady!==false||src.sourceLicenseVerified!==false
     ||src.archiveBytes!==info.size||!/^[a-f0-9]{64}$/.test(src.archiveSha256)
     ||src.headerZoomRangeConsistent!==true||!Number.isInteger(src.actualIndexedMaxZoom))
     throw Error('INVALID_SOURCE: untrusted local archive inspection failed');
   const jobId=randomUUID();
   pending.clear();
   pending.set(jobId,{path:sourcePath,sha:src.archiveSha256,selection,zoom:src.actualIndexedMaxZoom,
     expires:Date.now()+10*60*1000});
   return {canceled:false,jobId,selectionId:selection.selectionId,
     fileName:path.basename(sourcePath),sourceArchiveSha256:src.archiveSha256,
     sourceArchiveBytes:src.archiveBytes,zoom:src.actualIndexedMaxZoom,
     sourceAttributionFromArchive:typeof src.sourceAttributionFromArchive==='string'?
       src.sourceAttributionFromArchive.slice(0,2048):null,
     verticalDatumStatus:'unknown',sourceRightsStatus:'unknown',
     productionGenerationReady:false,redistributionApproved:false,policyVersion:POLICY};
 });
