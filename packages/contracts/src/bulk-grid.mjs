import {createHash} from 'node:crypto';
import {validateGrid} from './validation.mjs';

/** M2M-GRID-BIN-1: ASCII magic with NUL, LE u32 width/height/sampleCount,
 * then row-major [u8 marker, optional LE f64] per node. Immutable and bounded.
 * This function only processes bytes obtained via an AUTHORIZED NATIVE REGISTRY;
 * it must never accept a renderer-provided URL or filesystem pathname.
 */
const MAGIC=Buffer.from('M2M-GRID-BIN-1\0','ascii');
const MAX_SAMPLES=1_000_000;
export function decodeGridBinary(input){
 const data=Buffer.from(input);
 if(data.length<MAGIC.length+12||!data.subarray(0,MAGIC.length).equals(MAGIC))throw Error('INVALID_SOURCE: invalid grid binary magic');
 let p=MAGIC.length;
 const width=data.readUInt32LE(p);p+=4;
 const height=data.readUInt32LE(p);p+=4;
 const count=data.readUInt32LE(p);p+=4;
 if(width<2||height<2||width>4096||height>4096||width*height!==count||count>MAX_SAMPLES)throw Error('OUT_OF_MEMORY: invalid grid dimensions');
 const validMask=[],samplesMeters=[];
 for(let i=0;i<count;i++){
  if(p>=data.length)throw Error('INCOMPLETE_GRID: truncated grid flags');
  const marker=data[p++];
  if(marker===0){validMask.push(false);samplesMeters.push(null);}
  else if(marker===1){
   if(p+8>data.length)throw Error('INCOMPLETE_GRID: truncated elevation sample');
   const z=data.readDoubleLE(p);p+=8;
   if(!Number.isFinite(z))throw Error('NONFINITE_ELEVATION: invalid terrain sample');
   validMask.push(true);samplesMeters.push(Object.is(z,-0)?0:z);
  } else throw Error('INVALID_SOURCE: invalid grid flag');
 }
 if(p!==data.length)throw Error('INVALID_SOURCE: trailing grid binary bytes');
 return {width,height,validMask,samplesMeters};
}
export function encodeGridBinary(grid){
 const {width,height,samplesMeters,validMask}=grid;
 if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<2||height<2||width>4096||height>4096||width*height>MAX_SAMPLES||validMask?.length!==width*height||samplesMeters?.length!==width*height)
   throw Error('INCOMPLETE_GRID: invalid shape');
 const chunks=[MAGIC,Buffer.alloc(12)];
 chunks[1].writeUInt32LE(width,0);chunks[1].writeUInt32LE(height,4);chunks[1].writeUInt32LE(width*height,8);
 for(let i=0;i<samplesMeters.length;i++){
  const flag=validMask[i],z=samplesMeters[i];
  if(flag===false&&z===null)chunks.push(Buffer.from([0]));
  else if(flag===true&&Number.isFinite(z)){
   const b=Buffer.alloc(9);b.writeUInt8(1,0);b.writeDoubleLE(Object.is(z,-0)?0:z,1);chunks.push(b);
  }else throw Error('INCOMPLETE_GRID: mask/elevation mismatch');
 }
 return Buffer.concat(chunks);
}
export function hydrateGridFromVerifiedRegistry(control,readVerifiedBytes){
 if(control.storageMode!=='local-binary-token'||control.samplesMeters!==null||control.validMask!==null||
    !/^[0-9a-f]{64}$/.test(control.gridDataSha256||'')||control.gridDataToken!=='m2m-grid:'+control.gridDataSha256)
    return {ok:false,code:'INVALID_SOURCE',message:'Invalid immutable grid token envelope.'};
 if(typeof readVerifiedBytes!=='function')return {ok:false,code:'DATA_MISSING',message:'Verified local grid resolver missing.'};
 let bytes;
 try{bytes=readVerifiedBytes(control.gridDataToken);}
 catch{return {ok:false,code:'DATA_MISSING',message:'Grid token is not authorized or available.'};}
 if(!bytes)return {ok:false,code:'DATA_MISSING',message:'Grid token is not registered.'};
 const buf=Buffer.from(bytes);
 if(buf.length>9_000_027)return {ok:false,code:'OUT_OF_MEMORY',message:'Grid binary exceeds maximum size.'};
 const actual=createHash('sha256').update(buf).digest('hex');
 if(actual!==control.gridDataSha256)return {ok:false,code:'HASH_MISMATCH',message:'Grid binary SHA-256 mismatch.'};
 try{
  const decoded=decodeGridBinary(buf);
  if(decoded.width!==control.width||decoded.height!==control.height)
   return {ok:false,code:'INCOMPLETE_GRID',message:'Binary shape does not match signed grid envelope.'};
  // This is an in-process decoded representation, never a giant inline IPC message.
  const hydrated={...control,...decoded,storageMode:'inline-fixture',gridDataToken:null,gridDataSha256:null};
  const verified=validateGrid(hydrated);
  if(!verified.ok)return verified;
  return {ok:true,grid:hydrated,verifiedTransportSha256:actual};
 }catch(err){
  const message=String(err?.message||err);
  return {ok:false,code:message.split(':')[0],message};
 }
}
