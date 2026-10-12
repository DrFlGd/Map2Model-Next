// Renderer-side display helpers. Authoritative export admission is always native.
export const EMPTY_COVERAGE={type:'FeatureCollection',features:[]};
export function coveragePolygon(bbox){
  if(!Array.isArray(bbox)||bbox.length!==4||
     !bbox.every(v=>typeof v==='number'&&Number.isFinite(v))||
     bbox[0] < -180||bbox[2] > 180||bbox[1] < -60||bbox[3] > 60||
     bbox[0]>=bbox[2]||bbox[1]>=bbox[3]){
    return EMPTY_COVERAGE;
  }
  const [w,s,e,n]=bbox;
  return {type:'FeatureCollection',features:[{
    type:'Feature',properties:{kind:'approximate-PMTiles-header-envelope'},
    geometry:{type:'Polygon',coordinates:[[[w,s],[e,s],[e,n],[w,n],[w,s]]]}
  }]};
}
export function coverageMessage(assessment,hasSource,hasSelection){
  if(!hasSource)return 'Choose a local PMTiles file to see its declared map boundary.';
  if(!hasSelection)return 'Declared PMTiles boundary shown. Draw a rectangle inside the outline to check actual indexed tile coverage.';
  if(!assessment)return 'Checking selected rectangle against the PMTiles tile index…';
  if(assessment.status==='checking')return 'Checking indexed coverage for the drawn rectangle…';
  if(assessment.status==='inside'&&assessment.exportEligible===true)
    return 'INSIDE indexed coverage at selected zoom. Elevation pixels, source rights, datum and accuracy remain unverified.';
  if(assessment.status==='partial')
    return 'PARTIALLY COVERED: redraw entirely inside indexed tiles. Missing required tiles: '+assessment.missingRequiredTiles+'.';
  if(assessment.status==='outside')
    return assessment.zoomAvailable===false
      ? 'OUTSIDE: selected zoom has no indexed terrain tiles in this PMTiles archive.'
      : 'OUTSIDE indexed coverage. Move or shrink the rectangle inside available tiles.';
  return 'Coverage unconfirmed: '+(assessment.message||'please choose source again or reduce the rectangle')+'.';
}
export function coverageAdmitsExperiment(assessment,selection,source){
  return !!(assessment&&source&&selection&&
    assessment.status==='inside'&&assessment.exportEligible===true&&
    assessment.sourceArchiveSha256===source.sourceArchiveSha256&&
    assessment.jobId===source.jobId&&assessment.selectionId===selection.selectionId&&
    assessment.zoom===source.zoom&&assessment.missingRequiredTiles===0);
}

/**
 * Construct the exact one-use native export receipt from the validated DRAWN
 * rectangle. The source-first file-picker receipt intentionally has
 * selectionId=null, and must never be sent as an export selection identity.
 * The Rust/Electron main-process coverage and consent gate remains authoritative.
 */
export function approvedExperimentalExportPayload(assessment,selection,source,accepted){
  if(accepted!==true)
    throw Error('INVALID_REQUEST: explicit private experimental job consent required');
  if(!coverageAdmitsExperiment(assessment,selection,source))
    throw Error('INVALID_SELECTION: current source rectangle lacks matching indexed coverage');
  return Object.freeze({
    jobId:source.jobId,
    selectionId:selection.selectionId,
    accepted:true
  });
}
