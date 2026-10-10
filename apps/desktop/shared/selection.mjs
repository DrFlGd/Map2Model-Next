import {selectionDigest} from '../../../packages/contracts/src/canonical.mjs';
import {validateSelection} from '../../../packages/contracts/src/validation.mjs';

/** Contract-bound geographic selection. Input must be [west,south,east,north]. */
export function selectionFromBbox(bbox,options={}){
  const values=Array.isArray(bbox)?bbox.slice():null;
  const terrainProfile={
    targetGroundSampleMeters:options.targetGroundSampleMeters??12,
    noDataPolicy:'fail',
    maxSamples:options.maxSamples??1_000_000
  };
  const selection={
    schemaVersion:'1.0.0',kind:'SelectionV1',selectionId:'',planet:'earth',
    horizontalCrs:'EPSG:4326',bbox:values,terrainProfile
  };
  // Digest only after basic shape check; reject, never silently wrap/clamp.
  if(!values||values.length!==4||!values.every(v=>typeof v==='number'&&Number.isFinite(v)))
    throw Error('INVALID_SELECTION: four finite WGS84 bounds are required');
  selection.selectionId='sel-'+selectionDigest(selection).slice(0,32);
  const verdict=validateSelection(selection);
  if(!verdict.ok)throw Error(verdict.code+': '+verdict.message);
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
  // Prevent extra application-owned fields masquerading as a canonical selection.
  const permitted=['schemaVersion','kind','selectionId','planet','horizontalCrs','bbox','terrainProfile'];
  if(Object.keys(value).some(key=>!permitted.includes(key))||
    Object.keys(terrainProfile).some(key=>!['targetGroundSampleMeters','noDataPolicy','maxSamples'].includes(key)))
    throw Error('INVALID_SELECTION: unexpected contract field');
  return proposed;
}
