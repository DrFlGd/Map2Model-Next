import {createHash} from 'node:crypto';

/** Proposed deterministic V1 primitive encodings. Reference fixture hashes are independently computed. */
export class CanonicalWriter {
  constructor(prefix){this.parts=[Buffer.from(prefix,'ascii')];}
  bytes(v){this.parts.push(Buffer.from(v));return this;}
  u8(v){const b=Buffer.alloc(1);b.writeUInt8(v);return this.bytes(b);}
  u32(v){if(!Number.isSafeInteger(v)||v<0||v>4294967295)throw Error('invalid u32');const b=Buffer.alloc(4);b.writeUInt32LE(v);return this.bytes(b);}
  u64(v){if(!Number.isSafeInteger(v)||v<0)throw Error('invalid u64');const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(v));return this.bytes(b);}
  f64(v){if(!Number.isFinite(v))throw Error('non-finite f64');const b=Buffer.alloc(8);b.writeDoubleLE(Object.is(v,-0)?0:v);return this.bytes(b);}
  i64(v){if(!Number.isSafeInteger(v))throw Error('invalid i64');const b=Buffer.alloc(8);b.writeBigInt64LE(BigInt(v));return this.bytes(b);}
  str(v){if(typeof v!=='string')throw Error('invalid string');const b=Buffer.from(v,'utf8');return this.u32(b.length).bytes(b);}
  digest(){return createHash('sha256').update(Buffer.concat(this.parts)).digest('hex');}
}
function hash32(h){if(!/^[a-f0-9]{64}$/.test(h))throw Error('invalid SHA256');return Buffer.from(h,'hex');}
export function selectionDigest(v){
 const w=new CanonicalWriter('M2M-SELECTION-1\0');
 for(const val of v.bbox)w.f64(val);
 w.f64(v.terrainProfile.targetGroundSampleMeters)
  .str(v.terrainProfile.noDataPolicy).u32(v.terrainProfile.maxSamples);
 return w.digest();
}
export function manifestDigest(v){
 const w=new CanonicalWriter('M2M-MANIFEST-1\0');
 w.str(v.sourceName).str(v.releaseId).str(v.archiveId).bytes(hash32(v.sourceArchiveSha256));
 for(const val of v.coverageBbox)w.f64(val);
 w.str(v.horizontalCrs).str(v.verticalDatumStatus).str(v.verticalDatumId);
 w.f64(v.nominalGroundSampleMeters).str(v.nativeResolutionSource);
 if(v.uncertaintyMeters===null)w.u8(0);else w.u8(1).f64(v.uncertaintyMeters);
 w.str(v.coverageStatus);
 if(v.coverageIndexSha256===null)w.u8(0);else w.u8(1).bytes(hash32(v.coverageIndexSha256));
 w.u64(v.sourceBytes).str(v.licenseIdentifier).str(v.attribution).str(v.decoderVersion);
 return w.digest();
}
export function gridDigest(v){
 if(!Array.isArray(v.samplesMeters)||!Array.isArray(v.validMask)||v.samplesMeters.length!==v.width*v.height||v.validMask.length!==v.width*v.height)throw Error('grid samples/mask mismatch');
 const w=new CanonicalWriter('M2M-GRID-1\0');
 w.u32(v.width).u32(v.height).str(v.selectionId).bytes(hash32(v.sourceManifestSha256));
 w.str(v.horizontalTransformId).str(v.verticalDatumId);
 for(const q of [...v.anchorWgs84,v.minEastMeters,v.maxNorthMeters,v.dxMeters,v.dyMeters])w.f64(q);
 for(let i=0;i<v.samplesMeters.length;i++){
   if(v.validMask[i]===true)w.u8(1).f64(v.samplesMeters[i]);
   else if(v.validMask[i]===false && v.samplesMeters[i]===null)w.u8(0);
   else throw Error('mask value differs from sample');
 }
 return w.digest();
}
function vertexKey(v){return v.map(x=>{if(!Number.isFinite(x))throw Error('invalid mesh vertex'); const n=Math.sign(x)*Math.floor(Math.abs(x)*1e6+0.5);if(!Number.isSafeInteger(n))throw Error('mesh vertex coordinate overflow');return n;});}
function compare3(a,b){for(let i=0;i<3;i++){if(a[i]!==b[i])return a[i]-b[i];}return 0;}
function rotated(t){return [t,[t[1],t[2],t[0]],[t[2],t[0],t[1]]].sort(compare3)[0];}
export function canonicalMeshDigest(mesh){
 if(!Array.isArray(mesh.vertices)||!Array.isArray(mesh.triangles))throw Error('invalid mesh');
 const order=mesh.vertices.map((v,i)=>({v:vertexKey(v),i})).sort((a,b)=>compare3(a.v,b.v));
 for(let i=1;i<order.length;i++)if(compare3(order[i-1].v,order[i].v)===0)throw Error('duplicate quantized vertex');
 const indexes=new Map(order.map((v,i)=>[v.i,i]));
 const tris=mesh.triangles.map(({partId,indices})=>{
   if(typeof partId!=='string'||partId.length===0||indices?.length!==3||indices.some(i=>!Number.isInteger(i)||!indexes.has(i)))throw Error('invalid triangle');
   const canonical=rotated(indices.map(i=>indexes.get(i)));
   if(new Set(canonical).size!==3)throw Error('degenerate triangle indices');
   return {partId,indices:canonical};
 }).sort((a,b)=>Buffer.compare(Buffer.from(a.partId,'utf8'),Buffer.from(b.partId,'utf8'))||compare3(a.indices,b.indices));
 const w=new CanonicalWriter('M2M-MESH-1\0');
 w.u32(order.length);
 for(const {v} of order)for(const n of v)w.i64(n);
 w.u32(tris.length);
 for(const t of tris)w.str(t.partId).u32(t.indices[0]).u32(t.indices[1]).u32(t.indices[2]);
 return w.digest();
}
