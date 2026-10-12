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
   // Source picker intentionally works BEFORE a rectangle is drawn.
   const selection=null;
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
   // Agent 2's bounded native PMTiles index/metadata inspector is the
   // only map coverage authority. Never give a renderer the file path.
   let overlay;
   try{
     const checkedCoverage=await runNative(nativeEnginePath(),
       ['--coverage-local',sourcePath],
       {windowsHide:true,timeout:120000,maxBuffer:128*1024,encoding:'utf8'});
     overlay=JSON.parse(checkedCoverage.stdout).result;
   }catch{throw Error('INVALID_SOURCE: local PMTiles declared boundary inspection failed');}
   const bbox=overlay?.declaredCoverageBboxWgs84;
   if(overlay?.kind!=='LocalPMTilesCoveragePreviewV1'||
      overlay.sourceArchiveSha256!==src.archiveSha256||
      overlay.sourceArchiveBytes!==info.size||
      overlay.productionGenerationReady!==false||
      overlay.sourceLicenseVerified!==false||
      overlay.verticalDatumStatus!=='unknown'||
      overlay.networkRequests!==0||
      !Array.isArray(bbox)||bbox.length!==4||
      !bbox.every(Number.isFinite)||bbox[0]>=bbox[2]||bbox[1]>=bbox[3]||
      !Number.isInteger(overlay.headerMinZoom)||!Number.isInteger(overlay.headerMaxZoom)||
      src.actualIndexedMaxZoom<overlay.headerMinZoom||
      src.actualIndexedMaxZoom>overlay.headerMaxZoom)
     throw Error('INVALID_SOURCE: native coverage and selected source identity disagree');
   const jobId=randomUUID();
   pending.clear();
   pending.set(jobId,{path:sourcePath,sha:src.archiveSha256,selection,zoom:src.actualIndexedMaxZoom,
     expires:Date.now()+10*60*1000,revision:0,coverageStatus:null});
   return {canceled:false,jobId,selectionId:null,
     fileName:path.basename(sourcePath),sourceArchiveSha256:src.archiveSha256,
     sourceArchiveBytes:src.archiveBytes,zoom:src.actualIndexedMaxZoom,
     declaredCoverageBboxWgs84:bbox,coverageGeoJson:overlay.coverageGeoJson,
     headerMinZoom:overlay.headerMinZoom,headerMaxZoom:overlay.headerMaxZoom,
     indexedTileCount:overlay.indexedTileCount,
     boundarySource:overlay.boundarySource,
     sourceAttributionFromArchive:typeof src.sourceAttributionFromArchive==='string'?
       src.sourceAttributionFromArchive.slice(0,2048):null,
     verticalDatumStatus:'unknown',sourceRightsStatus:'unknown',
     productionGenerationReady:false,redistributionApproved:false,policyVersion:POLICY};
 });
 handler('assessExperimentalCoverage',async payload=>{
   const job=pending.get(payload?.jobId);
   if(!job||Date.now()>job.expires)
     throw Error('INVALID_REQUEST: local source receipt expired, choose file again');
   const {assertSelection}=await import('../shared/selection.mjs');
   const selected=assertSelection(payload.selection);
   // Invalidate the previous selection and consent even while native runs.
   const revision=++job.revision;
   job.selection=null;job.coverageStatus=null;
   let result;
   try{
     const argv=['--coverage-local',job.path,job.sha,
       ...selected.bbox.map(String),String(job.zoom)];
     const {stdout}=await runNative(nativeEnginePath(),argv,
       {windowsHide:true,timeout:120000,maxBuffer:128*1024,encoding:'utf8'});
     result=JSON.parse(stdout).result;
   }catch{throw Error('INVALID_SOURCE: selected local PMTiles coverage could not be checked');}
   if(pending.get(payload.jobId)!==job||job.revision!==revision)
     throw Error('INVALID_SELECTION: stale coverage check superseded by a newer selection');
   const status=result?.selection;
   if(result?.kind!=='LocalPMTilesCoveragePreviewV1'||
      result.sourceArchiveSha256!==job.sha||
      result.networkRequests!==0||result.productionGenerationReady!==false||
      status?.requestedZoom!==job.zoom||
      JSON.stringify(status.bboxWgs84)!==JSON.stringify(selected.bbox)||
      !['inside','partial','outside'].includes(status.status))
     throw Error('INVALID_SOURCE: coverage response does not match chosen source and rectangle');
   const inside=status.status==='inside'&&status.allRequiredTilesIndexed===true&&
     status.missingRequiredTiles===0&&status.zoomAvailable===true;
   if(inside){job.selection=selected;job.coverageStatus='inside';}
   return {jobId:payload.jobId,selectionId:selected.selectionId,
     sourceArchiveSha256:job.sha,zoom:job.zoom,
     status:status.status,zoomAvailable:status.zoomAvailable,
     requiredClosedBoundaryTiles:status.requiredClosedBoundaryTiles,
     indexedRequiredTiles:status.indexedRequiredTiles,
     missingRequiredTiles:status.missingRequiredTiles,
     classificationScope:status.classificationScope,
     exportEligible:inside,productionGenerationReady:false};
 });
 handler('exportExperimentalLocal',async payload=>{
   if(payload?.accepted!==true||typeof payload.jobId!=='string')
     throw Error('INVALID_REQUEST: explicit unchecked-by-default consent required');
   const job=pending.get(payload.jobId);
   pending.delete(payload.jobId); // consumed exactly once, even on cancellation/failure
   if(!job||Date.now()>job.expires)
     throw Error('INVALID_REQUEST: source selection or job consent expired');
   if(!job.selection||job.coverageStatus!=='inside'||
      payload.selectionId!==job.selection.selectionId)
     throw Error('INVALID_SELECTION: source rectangle lacks matching indexed tile coverage');
   // Recheck on native side immediately before user selects export location.
   // A changed archive or incomplete index NEVER becomes a published 3MF.
   let live;
   try{
     const {stdout}=await runNative(nativeEnginePath(),[
       '--coverage-local',job.path,job.sha,
       ...job.selection.bbox.map(String),String(job.zoom)
     ],{windowsHide:true,timeout:120000,maxBuffer:128*1024,encoding:'utf8'});
     live=JSON.parse(stdout).result;
   }catch{throw Error('HASH_MISMATCH: local PMTiles changed or became unreadable');}
   if(live?.sourceArchiveSha256!==job.sha||
      live?.selection?.status!=='inside'||live?.selection?.allRequiredTilesIndexed!==true||
      live?.selection?.missingRequiredTiles!==0)
     throw Error('DATA_MISSING: selected source is not fully indexed at the chosen zoom');
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
     try{await fs.link(staging,destination);}catch(error){
       throw Error(error?.code==='EEXIST'?
         'EXPORT_FAILED: atomic publication refused existing destination':
         'EXPORT_FAILED: atomic publication failed');
     }
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
