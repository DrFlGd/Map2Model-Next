import {selectionDigest, manifestDigest, gridDigest} from './canonical.mjs';

/** Candidate local linearized WGS84 tangent projection; G0 review is still required.
 * Accurate within bounded small-city extents; not a national-scale map CRS.
 */
const WGS84_A=6378137;
const WGS84_F=1/298.257223563;
const WGS84_E2=WGS84_F*(2-WGS84_F);
const RAD=Math.PI/180;
const MAX_PROJECTION_LATITUDE=60;
const MAX_SELECTION_METERS=2000;
const MAX_GRID_SAMPLES=1_000_000;
export function projectionScales(anchor){
 const [lon,lat]=anchor;
 if(!Number.isFinite(lon)||!Number.isFinite(lat)||Math.abs(lat)>MAX_PROJECTION_LATITUDE||lon < -180 || lon > 180)throw Error('INVALID_CRS: unsupported anchor');
 const phi=lat*RAD,t=1-WGS84_E2*Math.sin(phi)**2;
 return {eastPerDegree:WGS84_A*Math.cos(phi)/Math.sqrt(t)*RAD,
         northPerDegree:WGS84_A*(1-WGS84_E2)/t**1.5*RAD};
}
export function projectWgs84(anchor,point){
 const [lon,lat]=point;
 if(!Number.isFinite(lon)||!Number.isFinite(lat)||Math.abs(lat)>MAX_PROJECTION_LATITUDE||Math.abs(lon-anchor[0])>1)throw Error('INVALID_CRS: outside local projection domain');
 const {eastPerDegree,northPerDegree}=projectionScales(anchor);
 return [(lon-anchor[0])*eastPerDegree,(lat-anchor[1])*northPerDegree];
}
export function unprojectWgs84(anchor,[east,north]){
 const {eastPerDegree,northPerDegree}=projectionScales(anchor);
 if(!Number.isFinite(east)||!Number.isFinite(north)||Math.abs(east)>MAX_SELECTION_METERS||Math.abs(north)>MAX_SELECTION_METERS)throw Error('INVALID_CRS: projection extent too large');
 return [anchor[0]+east/eastPerDegree,anchor[1]+north/northPerDegree];
}
function fail(code,message,details={}){return {ok:false,code,message,details};}
function finite(v){return typeof v==='number'&&Number.isFinite(v);}
function bboxValid(b){
 return Array.isArray(b)&&b.length===4&&b.every(finite)&&
   b[0]>= -180&&b[2]<=180&&b[1]>= -MAX_PROJECTION_LATITUDE&&b[3]<=MAX_PROJECTION_LATITUDE&&
   b[0]<b[2]&&b[1]<b[3]&&(b[2]-b[0])<1;
}
function bboxMeters(b){
 const origin=[b[0],b[1]],e=projectWgs84(origin,[b[2],b[3]]);return e;
}
export function validateSelection(v){
 if(v.schemaVersion!=='1.0.0'||v.kind!=='SelectionV1'||v.planet!=='earth'||v.horizontalCrs!=='EPSG:4326')return fail('INVALID_SELECTION','Unsupported selection schema, planet or CRS.');
 if(!bboxValid(v.bbox))return fail('INVALID_SELECTION','Selection must be a non-wrapping, nonzero-area bbox within ±60°.');
 const [e,n]=bboxMeters(v.bbox);
 if(e>MAX_SELECTION_METERS||n>MAX_SELECTION_METERS)return fail('INVALID_SELECTION','G1 selection may not exceed 2 km on either axis.');
 if(!v.terrainProfile||v.terrainProfile.noDataPolicy!=='fail'||!finite(v.terrainProfile.targetGroundSampleMeters)||
   v.terrainProfile.targetGroundSampleMeters<0.1||v.terrainProfile.targetGroundSampleMeters>2000)
   return fail('INVALID_SELECTION','Unsupported elevation sample spacing or no-data policy.');
 if(!Number.isSafeInteger(v.terrainProfile.maxSamples)||v.terrainProfile.maxSamples>MAX_GRID_SAMPLES||v.terrainProfile.maxSamples<4)
   return fail('OUT_OF_MEMORY','Requested sample count exceeds G1 budget.');
 if(v.selectionId!=='sel-'+selectionDigest(v).slice(0,32))return fail('INVALID_SELECTION','Selection ID does not match canonical selection hash.');
 return {ok:true};
}
export function validateManifest(v){
 if(!bboxValid(v.coverageBbox))return fail('INVALID_CRS','Invalid manifest coverage envelope.');
 if(v.localAssetToken!=='m2m-asset:'+v.sourceArchiveSha256)return fail('HASH_MISMATCH','Archive token differs from archive SHA-256.');
 if(v.coverageStatus==='verified-tile-index'&&!v.coverageIndexSha256)return fail('DATA_MISSING','Verified coverage requires immutable index digest.');
 if(v.coverageStatus==='envelope-only'&&v.coverageIndexSha256!==null)return fail('INVALID_SOURCE','Unverified coverage cannot claim an index.');
 if(v.verticalDatumStatus!=='known')return fail('UNSUPPORTED_DATUM','Vertical datum not established: cannot mark terrain Ready.');
 if(v.manifestSha256!==manifestDigest(v))return fail('HASH_MISMATCH','Canonical manifest digest mismatch.');
 return {ok:true,indexDeclared:v.coverageStatus==='verified-tile-index'}; // Never means per-selection coverage verified.
}
export function validateGrid(v){
 if(v.schemaVersion!=='1.0.0'||v.kind!=='ElevationGridV1'||v.storageConvention!=='north-up-f64-row-major')
   return fail('INVALID_SOURCE','Unsupported grid schema or sample layout.');
 if(!Number.isSafeInteger(v.width)||!Number.isSafeInteger(v.height)||v.width<2||v.height<2||v.width>4096||v.height>4096)return fail('INCOMPLETE_GRID','Grid dimensions outside 2..4096.');
 const total=v.width*v.height;
 if(total>MAX_GRID_SAMPLES)return fail('OUT_OF_MEMORY','Grid exceeds 1 million G1 samples.');
 if(!finite(v.dxMeters)||!finite(v.dyMeters)||v.dxMeters<=0||v.dyMeters<=0)return fail('INVALID_CRS','Spacing must be positive and finite.');
 if(v.horizontalTransformId!=='WGS84-ENU-LINEARIZED-V1')return fail('INVALID_CRS','Unsupported local transform.');
 try{projectionScales(v.anchorWgs84);}catch{return fail('INVALID_CRS','Unsupported WGS84 anchor.');}
 if(!v.verticalDatumId||v.verticalDatumId==='unknown')return fail('UNSUPPORTED_DATUM','Grid datum unknown.');
 if(v.storageMode==='local-binary-token'){
   if(v.samplesMeters!==null||v.validMask!==null||v.gridDataToken!=='m2m-grid:'+v.gridDataSha256)return fail('INVALID_SOURCE','Grid must refer to immutable binary token.');
   return fail('DATA_MISSING','Grid bulk data must be resolved, hashed and validated before mesh generation.',{resolutionRequired:true});
 }
 if(v.storageMode!=='inline-fixture'||!Array.isArray(v.samplesMeters)||!Array.isArray(v.validMask)||v.samplesMeters.length!==total||v.validMask.length!==total)return fail('INCOMPLETE_GRID','Inline sample/mask lengths do not match dimensions.');
 for(let i=0;i<total;i++){
  if(typeof v.validMask[i]!=='boolean'|| (v.validMask[i]? !finite(v.samplesMeters[i]):v.samplesMeters[i]!==null))return fail('INCOMPLETE_GRID','Invalid sample/mask pairing.',{invalidSampleIndex:i});
 }
 try{if(gridDigest(v)!==v.gridSha256)return fail('INVALID_GRID_HASH','Canonical grid bytes do not match gridSha256.');}
 catch{return fail('INVALID_GRID_HASH','Grid cannot be canonically serialized.');}
 return {ok:true,missingSamples:v.validMask.filter(x=>!x).length};
}
export function validateBuild(v,grid){
 const [x0,y0,x1,y1]=v.clipRectMeters;
 if(![x0,y0,x1,y1,v.modelWidthMm,v.modelHeightMm,v.baseThicknessMm,v.minimumFeatureThicknessMm,v.verticalExaggeration,v.elevationZeroMeters].every(finite)||x0>=x1||y0>=y1||v.modelWidthMm<=0||v.modelHeightMm<=0||v.baseThicknessMm<=0||v.minimumFeatureThicknessMm<=0||v.verticalExaggeration<=0)return fail('INVALID_DIMENSIONS','Invalid bounds or print dimensions.');
 const sx=v.modelWidthMm/(x1-x0),sy=v.modelHeightMm/(y1-y0);
 if(Math.abs(sx-sy)>1e-9*Math.max(sx,sy))return fail('INVALID_DIMENSIONS','X/Y physical scaling must preserve aspect ratio.');
 if(grid){
   const val=validateGrid(grid);if(!val.ok)return val;
   if(v.gridSha256!==grid.gridSha256)return fail('HASH_MISMATCH','Request/grid hash mismatch.');
   if(v.selectionId!==grid.selectionId)return fail('HASH_MISMATCH','Selection identity differs from validated grid.');
   const gx1=grid.minEastMeters, gx2=gx1+(grid.width-1)*grid.dxMeters;
   const gy2=grid.maxNorthMeters, gy1=gy2-(grid.height-1)*grid.dyMeters;
   if(x0<gx1-1e-9||x1>gx2+1e-9||y0<gy1-1e-9||y1>gy2+1e-9)return fail('INVALID_DIMENSIONS','Clip exceeds grid node domain.');
   for(const [a,min,step] of [[x0,gx1,grid.dxMeters],[x1,gx1,grid.dxMeters],[y0,gy1,grid.dyMeters],[y1,gy1,grid.dyMeters]]){
     const f=(a-min)/step;if(Math.abs(f-Math.round(f))>1e-8)return fail('INVALID_DIMENSIONS','G1 requires grid-node-aligned clip edges.');
   }
   let lowest=Infinity,included=0;
   for(let row=0;row<grid.height;row++)for(let col=0;col<grid.width;col++){
     const gx=grid.minEastMeters+col*grid.dxMeters,gy=grid.maxNorthMeters-row*grid.dyMeters;
     if(gx<x0-1e-9||gx>x1+1e-9||gy<y0-1e-9||gy>y1+1e-9)continue;
     included++;
     const i=row*grid.width+col;
     if(!grid.validMask[i])return fail('DATA_MISSING','A clipped terrain sample is missing.',{invalidSampleIndex:i});
     lowest=Math.min(lowest,grid.samplesMeters[i]);
   }
   if(included<4)return fail('INVALID_DIMENSIONS','Clip must contain at least 2×2 nodes.');
   if(v.elevationZeroMeters>lowest+1e-9)return fail('INVALID_DIMENSIONS','Elevation baseline above included terrain.');
 }
 return {ok:true,xyMmPerMeter:sx,zMmForElevation:z=>v.baseThicknessMm+(z-v.elevationZeroMeters)*sx*v.verticalExaggeration};
}
export function validateMeshResult(v){
 const g=v.geometry,a=v.artifact;
 if(!g.closed||!g.oriented||g.nonManifoldEdges!==0||g.degenerateTriangles!==0||
  g.componentCount!==1||g.selfIntersectionStatus!=='passed'||g.validation.status!=='passed'||a.conformance!=='passed')
   return fail('INVALID_GEOMETRY','Invalid or unverified mesh must not produce success result.');
 const box=g.measuredAabbMm;
 if(!box.every(finite)||box[0]>=box[3]||box[1]>=box[4]||box[2]>=box[5]||g.volumeMm3<=0)return fail('INVALID_GEOMETRY','Measured AABB/volume invalid.');
 return {ok:true};
}
/** Successful exports are accepted only when bound to the original build request.
 * A standalone synthetic mesh result is NOT proof of a valid regional 3MF export.
 */
export function validateResultAgainstRequest(result,request,grid=null){
 const checked=validateMeshResult(result);if(!checked.ok)return checked;
 if(result.jobId!==request.jobId||result.artifact.exportToken!==request.exportToken)
   return fail('HASH_MISMATCH','Export result is not bound to the submitted build job/token.');
 const a=result.geometry.measuredAabbMm;
 const tolMm=0.05; // absolute XY bounding tolerance for G1 mm printing
 if(Math.abs(a[0])>tolMm||Math.abs(a[1])>tolMm||Math.abs(a[2])>tolMm||
    Math.abs((a[3]-a[0])-request.modelWidthMm)>tolMm||
    Math.abs((a[4]-a[1])-request.modelHeightMm)>tolMm)
   return fail('INVALID_DIMENSIONS','Exported XYZ bounds do not match requested physical XY footprint.');
 const required=Math.max(request.minimumFeatureThicknessMm,request.baseThicknessMm);
 if(!finite(result.geometry.minimumMeasuredThicknessMm)||result.geometry.minimumMeasuredThicknessMm+tolMm<required)
   return fail('INVALID_GEOMETRY','Measured minimum thickness below required print dimensions.');
 if(grid){
   const build=validateBuild(request,grid);if(!build.ok)return build;
   let maximum=-Infinity;
   const [x0,y0,x1,y1]=request.clipRectMeters;
   for(let row=0;row<grid.height;row++)for(let col=0;col<grid.width;col++){
     const x=grid.minEastMeters+col*grid.dxMeters,y=grid.maxNorthMeters-row*grid.dyMeters;
     if(x<x0-1e-9||x>x1+1e-9||y<y0-1e-9||y>y1+1e-9)continue;
     maximum=Math.max(maximum,build.zMmForElevation(grid.samplesMeters[row*grid.width+col]));
   }
   if(!finite(maximum)||Math.abs(a[5]-maximum)>tolMm)
      return fail('INVALID_DIMENSIONS','Exported Z maximum does not match the clipped elevation grid.');
 }
 return {ok:true};
}
export function validateJobState(v){
 const terminal=['succeeded','failed','cancelled'].includes(v.status);
 if(v.status==='succeeded'&&(v.reasonCode!==null||v.progress01!==1||v.phase!=='complete'))return fail('INTERNAL_ERROR','False success state.');
 if(v.status==='failed'&&(!v.reasonCode||v.reasonCode==='CANCELLED'))return fail('INTERNAL_ERROR','Failed job missing an actual reason.');
 if(v.status==='cancelled'&&(v.phase!=='cancel'||v.reasonCode!=='CANCELLED'))return fail('INTERNAL_ERROR','Invalid cancellation terminal.');
 if(!terminal&&v.status==='cancel_requested'&&v.phase==='complete')return fail('INTERNAL_ERROR','Cancellation acknowledgement cannot mark job completed.');
 return {ok:true};
}
export function validJobTransition(previous,next){
 if(previous.jobId!==next.jobId||previous.correlationId!==next.correlationId)return false;
 if(['succeeded','failed','cancelled'].includes(previous.status))return false;
 if(next.progress01+1e-12<previous.progress01)return false;
 const allowed={queued:['running','cancel_requested','failed','cancelled'],running:['running','cancel_requested','succeeded','failed','cancelled'],cancel_requested:['cancel_requested','cancelled','failed']};
 return (allowed[previous.status]||[]).includes(next.status)&&validateJobState(next).ok;
}
