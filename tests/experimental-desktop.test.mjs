import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const {registerExperimentalHandlers}=createRequire(import.meta.url)('../apps/desktop/electron/experimental.cjs');

test('G2 experimental handlers are main-only and registered as distinct operations',()=>{
  const operations=new Map();
  registerExperimentalHandlers({
    handler:(name,fn)=>operations.set(name,fn),
    app:{getPath:()=>'/tmp'},
    dialog:{},
    getWindow:()=>null,
    nativeEnginePath:()=>'/native',
    runNative:async()=>{throw Error('native runner not expected');}
  });
  assert.equal(typeof operations.get('selectExperimentalSource'),'function');
  assert.equal(typeof operations.get('exportExperimentalLocal'),'function');
  assert.equal(typeof operations.get('discardExperimentalSource'),'function');
});

test('G2 default-unauthorized export cannot summon native file Save dialog',async()=>{
  let saves=0;
  const ops=new Map();
  registerExperimentalHandlers({
    handler:(name,fn)=>ops.set(name,fn),
    app:{getPath:()=>'/tmp'},
    dialog:{showSaveDialog:async()=>{saves++;return {canceled:true};}},
    getWindow:()=>null,
    nativeEnginePath:()=>'/native',
    runNative:async()=>{throw Error('native runner not expected');}
  });
  await assert.rejects(ops.get('exportExperimentalLocal')({jobId:'x',accepted:false}),/consent/);
  await assert.rejects(ops.get('exportExperimentalLocal')({jobId:'x',accepted:true}),/expired/);
  assert.equal(saves,0);
});

test('G2 original local flow publishes only after SHA, denies overwrite and prevents consent replay',async()=>{
 const fs=await import('node:fs/promises');
 const os=await import('node:os');
 const path=await import('node:path');
 const {createHash}=await import('node:crypto');
 const {selectionFromBbox}=await import('../apps/desktop/shared/selection.mjs');
 const root=await fs.mkdtemp(path.join(os.tmpdir(),'m2m-g2-handlers-'));
 try{
  const input=path.join(root,'source.pmtiles'), dest=path.join(root,'output.3mf');
  await fs.writeFile(input,Buffer.alloc(128,65));
  const selection=selectionFromBbox([-84.024,39.7067,-84.023,39.7077]);
  const digest=b=>createHash('sha256').update(b).digest('hex');
  const bytes=Buffer.from('original synthetic test-only native result');
  const sourceSha='a'.repeat(64),ops=new Map();
  registerExperimentalHandlers({
   handler:(name,fn)=>ops.set(name,fn),
   app:{getPath:()=>root},getWindow:()=>null,nativeEnginePath:()=>'/native',
   dialog:{
    showOpenDialog:async()=>({canceled:false,filePaths:[input]}),
    showSaveDialog:async()=>({canceled:false,filePath:dest})
   },
   runNative:async(_binary,args)=>{
    if(args[0]==='--inspect-local-experimental')return {stdout:JSON.stringify({status:'ok',result:{
     inspectionKind:'UnverifiedLocalPMTilesFormatV1',archiveBytes:128,
     archiveSha256:sourceSha,headerZoomRangeConsistent:true,
     actualIndexedMaxZoom:12,sourceLicenseVerified:false,productionGenerationReady:false
    }})};
    await fs.writeFile(args[7],bytes,{flag:'wx'});
    return {stdout:JSON.stringify({status:'ok',result:{
     experimentalOnly:true,productionGenerationReady:false,redistributionApproved:false,
     threeMfExported:true,jobId:args[5],selectionId:selection.selectionId,
     sourceArchiveSha256:sourceSha,packageSha256:digest(bytes),bytes:bytes.length,
     candidateHeightsSha256:'b'.repeat(64),vertices:96,triangles:188,aabbMm:[0,0,0,120,100,50]
    }})};
   }
  });
  const receipt=await ops.get('selectExperimentalSource')({selection});
  assert(!JSON.stringify(receipt).includes(input));
  const exported=await ops.get('exportExperimentalLocal')({jobId:receipt.jobId,
    selectionId:selection.selectionId,accepted:true});
  assert.equal(exported.packageSha256,digest(bytes));
  assert.deepEqual(await fs.readFile(dest),bytes);
  await assert.rejects(ops.get('exportExperimentalLocal')({jobId:receipt.jobId,
    selectionId:selection.selectionId,accepted:true}),/expired/);
  const second=await ops.get('selectExperimentalSource')({selection});
  await assert.rejects(ops.get('exportExperimentalLocal')({jobId:second.jobId,
    selectionId:selection.selectionId,accepted:true}),/atomic publication/);
  assert.deepEqual(await fs.readFile(dest),bytes);
 }finally{await fs.rm(root,{recursive:true,force:true});}
});
