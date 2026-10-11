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
