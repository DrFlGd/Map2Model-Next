import {sha256} from '@noble/hashes/sha256';

/** Browser-safe, synchronous implementation of the frozen SelectionV1 digest.
 * Wire reference: ADR-004 M2M-SELECTION-1\0 (binary64 LE + length-prefixed strings).
 * CI compares the digest with the canonical Node contract implementation.
 */
const encoder=new TextEncoder();
const RADIANS=Math.PI/180;
const A=6378137;
const F=1/298.257223563;
const E2=F*(2-F);
function u32(v){
  const bytes=new Uint8Array(4);new DataView(bytes.buffer).setUint32(0,v,true);return bytes;
}
function f64(v){
  const bytes=new Uint8Array(8);new DataView(bytes.buffer).setFloat64(0,Object.is(v,-0)?0:v,true);return bytes;
}
function string(v){
  const bytes=encoder.encode(v);return [u32(bytes.length),bytes];
}
function hashSelection(bbox,terrainProfile){
  const parts=[encoder.encode('M2M-SELECTION-1\0'),...bbox.map(f64),
    f64(terrainProfile.targetGroundSampleMeters),...string(terrainProfile.noDataPolicy),u32(terrainProfile.maxSamples)];
  const total=parts.reduce((n,b)=>n+b.length,0),bytes=new Uint8Array(total);
  let offset=0;for(const part of parts){bytes.set(part,offset);offset+=part.length;}
  return Array.from(sha256(bytes),byte=>byte.toString(16).padStart(2,'0')).join('');
}
function validSelection(selection){
  const {bbox,terrainProfile}=selection;
  if(!Array.isArray(bbox)||bbox.length!==4||bbox.some(n=>typeof n!=='number'||!Number.isFinite(n)))
    throw Error('INVALID_SELECTION: four finite WGS84 bounds are required');
  const [west,south,east,north]=bbox;
  if(west< -180||east>180||south< -60||north>60||!(west<east)||!(south<north)||east-west>=1)
    throw Error('INVALID_SELECTION: non-wrapping bounds within ±60° are required');
  const phi=south*RADIANS;
  const q=1-E2*Math.sin(phi)**2;
  const width=(east-west)*RADIANS*A*Math.cos(phi)/Math.sqrt(q);
  const height=(north-south)*RADIANS*A*(1-E2)/q**1.5;
  if(width>2000||height>2000)throw Error('INVALID_SELECTION: maximum supported selection is 2 km per axis');
  if(terrainProfile.noDataPolicy!=='fail'||
     !Number.isFinite(terrainProfile.targetGroundSampleMeters)||
     terrainProfile.targetGroundSampleMeters<0.1||terrainProfile.targetGroundSampleMeters>2000)
    throw Error('INVALID_SELECTION: unsupported elevation sampling settings');
  if(!Number.isSafeInteger(terrainProfile.maxSamples)||
     terrainProfile.maxSamples<4||terrainProfile.maxSamples>1_000_000)
    throw Error('OUT_OF_MEMORY: sample budget outside G1 limits');
}
export function selectionFromBbox(bbox,options={}){
  const terrainProfile={
    targetGroundSampleMeters:options.targetGroundSampleMeters??12,
    noDataPolicy:'fail',
    maxSamples:options.maxSamples??1_000_000
  };
  const selection={
    schemaVersion:'1.0.0',kind:'SelectionV1',selectionId:'',planet:'earth',
    horizontalCrs:'EPSG:4326',bbox:Array.isArray(bbox)?bbox.slice():bbox,terrainProfile
  };
  validSelection(selection);
  selection.selectionId='sel-'+hashSelection(selection.bbox,selection.terrainProfile).slice(0,32);
  return selection;
}
export function assertSelection(value){
  if(!value||typeof value!=='object'||Array.isArray(value))
    throw Error('INVALID_SELECTION: expected contract object');
  const {schemaVersion,kind,selectionId,planet,horizontalCrs,bbox,terrainProfile}=value;
  if(schemaVersion!=='1.0.0'||kind!=='SelectionV1'||planet!=='earth'||horizontalCrs!=='EPSG:4326')
    throw Error('INVALID_SELECTION: unexpected schema, planet or reference');
  if(typeof selectionId!=='string'||!terrainProfile||typeof terrainProfile!=='object')
    throw Error('INVALID_SELECTION: required identity or terrain profile absent');
  const proposed=selectionFromBbox(bbox,{
    targetGroundSampleMeters:terrainProfile.targetGroundSampleMeters,
    maxSamples:terrainProfile.maxSamples
  });
  if(terrainProfile.noDataPolicy!=='fail'||selectionId!==proposed.selectionId)
    throw Error('INVALID_SELECTION: unsupported policy or mismatched digest');
  const permitted=['schemaVersion','kind','selectionId','planet','horizontalCrs','bbox','terrainProfile'];
  if(Object.keys(value).some(key=>!permitted.includes(key))||
    Object.keys(terrainProfile).some(key=>!['targetGroundSampleMeters','noDataPolicy','maxSamples'].includes(key)))
    throw Error('INVALID_SELECTION: unexpected contract field');
  return proposed;
}
