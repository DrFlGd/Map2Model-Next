'use strict';
const fs=require('node:fs/promises');
const path=require('node:path');
const {createHash,randomUUID}=require('node:crypto');
const {execFile}=require('node:child_process');
const {promisify}=require('node:util');
const runBinary=promisify(execFile);
const POLICY='PRIVATE-LOCAL-TERRAIN-2026-10-11';
function registerExperimentalHandlers({handler,app,dialog,getWindow,nativeEnginePath,runNative=runBinary}){
 const pending=new Map();
 const running=new Map();
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
     ({stdout}=await runNative(nativeEnginePath(),['--inspect-local-experimental',sourcePath],
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
 handler('exportExperimentalLocal',async payload=>{
   if(payload?.accepted!==true||typeof payload.jobId!=='string')
     throw Error('INVALID_REQUEST: explicit unchecked-by-default consent required');
   const job=pending.get(payload.jobId);
   pending.delete(payload.jobId); // consumed exactly once, even on cancellation/failure
   if(!job||Date.now()>job.expires)
     throw Error('INVALID_REQUEST: source selection or job consent expired');
   if(payload.selectionId!==job.selection.selectionId)
     throw Error('INVALID_SELECTION: source/selection consent mismatch');
   const answer=await dialog.showSaveDialog(getWindow(),{
     title:'Save PRIVATE EXPERIMENTAL, unverified terrain 3MF',
     defaultPath:'Map2Model-PRIVATE-EXPERIMENTAL.3mf',
     filters:[{name:'3D Manufacturing Format',extensions:['3mf']}],
     properties:['showOverwriteConfirmation']});
   if(answer.canceled||!answer.filePath)return {canceled:true};
   const destination=path.resolve(answer.filePath);
   if(path.extname(destination).toLowerCase()!=='.3mf')
     throw Error('INVALID_REQUEST: only native .3mf output allowed');
   const controller=new AbortController();
   const staging=path.join(path.dirname(destination),'.m2m-experimental-'+randomUUID()+'.stage.3mf');
   running.set(payload.jobId,controller);
   const temp=await fs.mkdtemp(path.join(app.getPath('temp'),'m2m-private-job-'));
   try{
     const selectionFile=path.join(temp,'selection.json');
     await fs.writeFile(selectionFile,JSON.stringify(job.selection),{mode:0o600,flag:'wx'});
     let stdout;
     try{
       ({stdout}=await runNative(nativeEnginePath(),[
         '--export-local-experimental',job.path,job.sha,selectionFile,
         String(job.zoom),payload.jobId,POLICY,staging
       ],{windowsHide:true,timeout:120000,maxBuffer:65536,encoding:'utf8',signal:controller.signal}));
     }catch{throw Error(controller.signal.aborted?'CANCELLED: experimental worker stopped':'EXPORT_FAILED: native experimental 3MF generation failed');}
     let response;
     try{response=JSON.parse(stdout);}catch{throw Error('EXPORT_FAILED: invalid native report');}
     const result=response.result;
     if(response.status!=='ok'||result?.experimentalOnly!==true
        ||result.productionGenerationReady!==false||result.redistributionApproved!==false
        ||result.threeMfExported!==true||result.jobId!==payload.jobId
        ||result.selectionId!==job.selection.selectionId||result.sourceArchiveSha256!==job.sha)
       throw Error('EXPORT_FAILED: unverified source/consent result mismatch');
     const info=await fs.lstat(staging);
     if(!info.isFile()||info.isSymbolicLink()||info.size>80*1024*1024)
       throw Error('EXPORT_FAILED: temporary model is not an ordinary 3MF');
     const bytes=await fs.readFile(staging);
     const sha=createHash('sha256').update(bytes).digest('hex');
     if(sha!==result.packageSha256||bytes.length!==result.bytes)
       throw Error('HASH_MISMATCH: native 3MF output altered');
     if(controller.signal.aborted)throw Error('CANCELLED: before publication');
     await fs.link(staging,destination);
     const finalBytes=await fs.readFile(destination);
     if(createHash('sha256').update(finalBytes).digest('hex')!==sha){
       await fs.unlink(destination).catch(()=>{});
       throw Error('HASH_MISMATCH: published 3MF checksum differs');
     }
     return {canceled:false,experimentalOnly:true,productionGenerationReady:false,
       fileName:path.basename(destination),packageSha256:sha,
       sourceArchiveSha256:job.sha,candidateHeightsSha256:result.candidateHeightsSha256,
       vertices:result.vertices,triangles:result.triangles,aabbMm:result.aabbMm};
   }finally{
     running.delete(payload.jobId);
     await fs.unlink(staging).catch(()=>{});
     await fs.rm(temp,{recursive:true,force:true});
   }
 });
 handler('cancelExperimentalJob',async payload=>{
   if(typeof payload?.jobId!=='string')return {cancelRequested:false};
   const controller=running.get(payload.jobId);
   if(!controller)return {cancelRequested:false};
   controller.abort();
   return {cancelRequested:true};
 });
 handler('discardExperimentalSource',async payload=>{
   if(typeof payload?.jobId==='string')pending.delete(payload.jobId);
   return {discarded:true};
 });
}
module.exports={registerExperimentalHandlers};
