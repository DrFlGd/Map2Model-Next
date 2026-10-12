import React,{useEffect,useRef,useState} from 'react';
import maplibregl from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import './styles.css';
import {selectionFromBbox} from '../shared/selection.mjs';
import {coveragePolygon,coverageMessage,coverageAdmitsExperiment,approvedExperimentalExportPayload} from '../shared/coverage.mjs';

const EMPTY_STYLE={version:8,sources:{},layers:[{id:'local-background',type:'background',paint:{'background-color':'#edf2ed'}}]};
const emptyStatus={offline:true,networkEnabled:false,libraryConfigured:false,libraryLabel:null,
  basemap:{status:'not-installed'},terrain:{status:'not-installed'},engine:{status:'not-installed',canGenerate:false}};
function longitudeLabel(lng){return lng.toFixed(5)+'° E';}
function latitudeLabel(lat){return lat.toFixed(5)+'° N';}
function graticule(){
 const features=[];
 for(let lon=-180;lon<=180;lon+=10){
  features.push({type:'Feature',geometry:{type:'LineString',coordinates:[[-179.999+Math.max(0,lon+179.999),-85],[Math.min(179.999,lon),85]]},properties:{}});}
 // Explicit valid vertical and horizontal lines for a coordinate-only offline style.
 const lines=[];
 for(let lon=-170;lon<=170;lon+=10)lines.push({type:'Feature',geometry:{type:'LineString',coordinates:[[lon,-60],[lon,60]]}});
 for(let lat=-60;lat<=60;lat+=10)lines.push({type:'Feature',geometry:{type:'LineString',coordinates:[[-179.9,lat],[179.9,lat]]}});
 return {type:'FeatureCollection',features:lines};
}
function selectedGeo(selection){
 if(!selection)return {type:'FeatureCollection',features:[]};
 const [west,south,east,north]=selection.bbox;
 return {type:'FeatureCollection',features:[{type:'Feature',geometry:{type:'Polygon',coordinates:[[[west,south],[east,south],[east,north],[west,north],[west,south]]]}}]};
}
function MapPane({selection,coverage,onSelection,onError,projectId}){
 const holder=useRef(null),mapRef=useRef(null),drawing=useRef(false),origin=useRef(null),drawEnabled=useRef(false);
 const selectionHandler=useRef(onSelection),errorHandler=useRef(onError),selectionRef=useRef(selection),coverageRef=useRef(coverage);
 const [draw,setDraw]=useState(false);
 const [focus,setFocus]=useState({lon:'-84.09',lat:'39.74'});
 const [center,setCenter]=useState('Coordinate reference • no offline basemap installed');
 selectionHandler.current=onSelection;errorHandler.current=onError;selectionRef.current=selection;coverageRef.current=coverage;
 useEffect(()=>{drawEnabled.current=draw;mapRef.current?.getCanvas().style.setProperty('cursor',draw?'crosshair':'grab');},[draw]);
 useEffect(()=>{
  const map=new maplibregl.Map({container:holder.current,style:EMPTY_STYLE,
    center:[-84.09,39.74],zoom:11,attributionControl:false,
    pitchWithRotate:false,dragRotate:false,renderWorldCopies:false,maxPitch:0,maxZoom:18});
  mapRef.current=map;
  const sourceUpdate=value=>{
   const source=map.getSource('terrain-selection');
   if(source)source.setData(selectedGeo(value));
  };
  const updateStatus=()=>{
   const p=map.getCenter();
   setCenter('Center '+longitudeLabel(p.lng)+' · '+latitudeLabel(p.lat)+' · Zoom '+map.getZoom().toFixed(1));
  };
  const down=event=>{
   if(!drawEnabled.current)return;
   drawing.current=true;origin.current=[event.lngLat.lng,event.lngLat.lat];
   event.preventDefault();map.dragPan.disable();
  };
  const move=event=>{
   if(!drawing.current||!origin.current)return;
   const [x,y]=origin.current,lon=event.lngLat.lng,lat=event.lngLat.lat;
   if(![lon,lat,x,y].every(Number.isFinite))return;
   sourceUpdate({bbox:[Math.min(x,lon),Math.min(y,lat),Math.max(x,lon),Math.max(y,lat)]});
  };
  const up=event=>{
   if(!drawing.current||!origin.current)return;
   drawing.current=false;map.dragPan.enable();
   const [x,y]=origin.current,lon=event.lngLat.lng,lat=event.lngLat.lat;
   origin.current=null;
   try{
    selectionHandler.current(selectionFromBbox([Math.min(x,lon),Math.min(y,lat),Math.max(x,lon),Math.max(y,lat)]));
    errorHandler.current('');
   }catch(err){
    errorHandler.current(String(err.message||err));
    sourceUpdate(selectionRef.current);
   }
  };
  map.on('load',()=>{
   map.addSource('local-grid',{type:'geojson',data:graticule()});
   map.addLayer({id:'local-grid-lines',type:'line',source:'local-grid',paint:{'line-color':'#bacdc8','line-width':0.8}});
   map.addSource('local-pmtiles-envelope',{type:'geojson',
     data:coveragePolygon(coverageRef.current?.declaredCoverageBboxWgs84)});
   map.addLayer({id:'local-pmtiles-envelope-fill',type:'fill',source:'local-pmtiles-envelope',
     paint:{'fill-color':'#287dcc','fill-opacity':0.08}});
   map.addLayer({id:'local-pmtiles-envelope-line',type:'line',source:'local-pmtiles-envelope',
     paint:{'line-color':'#216caf','line-width':3,'line-dasharray':[3,2]}});
   map.addSource('terrain-selection',{type:'geojson',data:selectedGeo(selectionRef.current)});
   map.addLayer({id:'terrain-selection-fill',type:'fill',source:'terrain-selection',paint:{'fill-color':'#21a89d','fill-opacity':0.22}});
   map.addLayer({id:'terrain-selection-line',type:'line',source:'terrain-selection',paint:{'line-color':'#007e73','line-width':2.5}});
   updateStatus();
  });
  map.on('moveend',updateStatus);map.on('mousedown',down);map.on('mousemove',move);map.on('mouseup',up);
  map.on('mouseleave',()=>{if(drawing.current){drawing.current=false;origin.current=null;map.dragPan.enable();sourceUpdate(selectionRef.current);}});
  return ()=>{map.remove();mapRef.current=null;};
 },[]);
 useEffect(()=>{
  const map=mapRef.current,source=map?.getSource('terrain-selection');
  if(source)source.setData(selectedGeo(selection));
 },[selection]);
 useEffect(()=>{
  const map=mapRef.current;
  if(!map)return;
  const source=map.getSource('local-pmtiles-envelope');
  if(source)source.setData(coveragePolygon(coverage?.declaredCoverageBboxWgs84));
  const bbox=coverage?.declaredCoverageBboxWgs84;
  if(coveragePolygon(bbox).features.length===1){
   map.fitBounds([[bbox[0],bbox[1]],[bbox[2],bbox[3]]],
     {padding:65,maxZoom:14,duration:0});
  }
 },[coverage?.sourceArchiveSha256]);
 useEffect(()=>{
  if(!selection||!mapRef.current)return;
  const map=mapRef.current,[west,south,east,north]=selection.bbox;
  map.fitBounds([[west,south],[east,north]],{padding:80,maxZoom:15,duration:0});
 },[projectId]);
 const jump=()=>{
  const lon=Number(focus.lon),lat=Number(focus.lat);
  if(!Number.isFinite(lon)||!Number.isFinite(lat)||Math.abs(lat)>60||Math.abs(lon)>180){
   onError('INVALID_CRS: enter longitude ±180 and latitude ±60');return;}
  mapRef.current?.jumpTo({center:[lon,lat],zoom:13});
  onError('');
 };
 return <section className="maparea" aria-label="Local geographic coordinate map">
   <div className="map-toolbar">
    <button type="button" className={draw?'primary':'secondary'} onClick={()=>setDraw(v=>!v)} aria-pressed={draw}>
      {draw?'Drawing enabled — drag a rectangle':'Draw terrain rectangle'}
    </button>
    <label>Longitude <input aria-label="Jump longitude" inputMode="decimal" value={focus.lon} onChange={e=>setFocus(p=>({...p,lon:e.target.value}))}/></label>
    <label>Latitude <input aria-label="Jump latitude" inputMode="decimal" value={focus.lat} onChange={e=>setFocus(p=>({...p,lat:e.target.value}))}/></label>
    <button type="button" onClick={jump}>Go to coordinates</button>
   </div>
   <div className="map-host" ref={holder}/>
   <div className="map-bottom"><span>{center}</span>
     <span>{coverage?'Blue dashed = approximate PMTiles header · Green = selection':'No external map tiles are requested'}</span>
   </div>
 </section>;
}
const guest={getState:async()=>emptyStatus};
export function App(){
 const api=window.map2model||guest;
 const [system,setSystem]=useState(emptyStatus),[projects,setProjects]=useState([]),[project,setProject]=useState(null),
   [selection,setSelection]=useState(null),[message,setMessage]=useState(''),[busy,setBusy]=useState(false),[projectName,setProjectName]=useState(''),
    [experimentalSource,setExperimentalSource]=useState(null),[experimentalAccepted,setExperimentalAccepted]=useState(false),
    [coverageStatus,setCoverageStatus]=useState(null);
 async function refresh(){
  const state=await api.getState();setSystem(state);
  if(state.libraryConfigured&&api.listProjects)setProjects(await api.listProjects());
 }
 useEffect(()=>{refresh().catch(e=>setMessage(String(e.message||e)));},[]);
 const action=async operation=>{
  setBusy(true);setMessage('');
  try{await operation();}
  catch(error){setMessage(String(error.message||error));}
  finally{setBusy(false);}
 };
 const clearExperimental=()=>{
   if(experimentalSource&&api.discardExperimentalSource)
     void api.discardExperimentalSource(experimentalSource.jobId).catch(()=>{});
   setExperimentalSource(null);setExperimentalAccepted(false);setCoverageStatus(null);
 };
 const choose=()=>action(async()=>{setSystem(await api.chooseLibrary());await refresh();});
 const create=()=>action(async()=>{
  const record=await api.createProject(projectName||'New terrain project');
  clearExperimental();setProject(record);setSelection(record.selection);setProjectName(record.title);await refresh();
 });
 const open=id=>action(async()=>{const record=await api.loadProject(id);
   clearExperimental();setProject(record);setSelection(record.selection);setProjectName(record.title);
 });
 const save=()=>action(async()=>{
  if(!project)throw Error('INVALID_PROJECT: create or open a project before saving');
  const record=await api.saveProject({projectId:project.projectId,title:projectName,selection});
  setProject(record);await refresh();setMessage('Saved locally. No network access was needed.');
 });
 const pick=next=>{
   setSelection(next);setCoverageStatus(null);setExperimentalAccepted(false);
   setMessage('Selection changed — checking local PMTiles coverage.');
 };
 useEffect(()=>{
   if(!experimentalSource||!selection){
     setCoverageStatus(null);setExperimentalAccepted(false);return;
   }
   if(!api.assessExperimentalCoverage){
     setCoverageStatus({status:'unknown',message:'Local tile index checker is unavailable'});
     return;
   }
   let live=true;
   setCoverageStatus({status:'checking'});setExperimentalAccepted(false);
   const timer=setTimeout(()=>{
     api.assessExperimentalCoverage(experimentalSource.jobId,selection)
       .then(result=>{
         if(!live)return;
         if(result.jobId!==experimentalSource.jobId||
            result.selectionId!==selection.selectionId||
            result.sourceArchiveSha256!==experimentalSource.sourceArchiveSha256)
           throw Error('Coverage response is stale');
         setCoverageStatus(result);
       })
       .catch(error=>{
         if(live)setCoverageStatus({status:'unknown',message:String(error.message||error)});
       });
   },180);
   return ()=>{live=false;clearTimeout(timer);};
 },[experimentalSource?.jobId,selection?.selectionId]);
 const chooseExperimental=()=>action(async()=>{
   if(!api.selectExperimentalSource)throw Error('DATA_MISSING: local PMTiles picker unavailable');
   setExperimentalSource(null);setExperimentalAccepted(false);
   const source=await api.selectExperimentalSource();
   if(source.canceled){setMessage('Local source selection cancelled.');return;}
   setExperimentalSource(source);
   setCoverageStatus(null);
   setMessage('Local source loaded. Its approximate blue boundary is on the map; draw or adjust the green rectangle within it.');
 });
 const exportExperimental=()=>action(async()=>{
   const request=approvedExperimentalExportPayload(
     coverageStatus,selection,experimentalSource,experimentalAccepted);
   setExperimentalSource(null);setExperimentalAccepted(false);
   const result=await api.exportExperimentalLocal(
     request.jobId,request.selectionId,request.accepted);
   if(result.canceled){setMessage('Experimental export cancelled.');return;}
   setMessage('EXPERIMENTAL PRIVATE 3MF '+result.fileName+' · '+result.vertices+
     ' vertices · archive SHA '+result.sourceArchiveSha256+' · package SHA '+result.packageSha256);
 });
 const exportOriginalPmtiles=()=>action(async()=>{
   if(!api.exportOriginalSyntheticPmtiles)throw Error('DATA_MISSING: synthetic PMTiles worker is unavailable');
   const result=await api.exportOriginalSyntheticPmtiles();
   if(result.canceled){setMessage('Synthetic PMTiles 3MF export cancelled.');return;}
   setMessage('Verified ORIGINAL SYNTHETIC 512px PMTiles → '+result.gridWidth+'×'+result.gridHeight+
     ' ElevationGridV1 → '+result.vertices+' mesh vertices → '+result.fileName+
     ' · source SHA '+result.sourceArchiveSha256+' · grid SHA '+result.gridSha256+
     ' · 3MF SHA '+result.packageSha256);
 });
 const exportSample=()=>action(async()=>{
   if(!api.exportSynthetic)throw Error('DATA_MISSING: native synthetic 3MF export unavailable');
   const result=await api.exportSynthetic();
   if(result.canceled){setMessage('Export canceled; no model written.');return;}
   setMessage('Saved verified SYNTHETIC 3MF: '+result.fileName+' · '+result.vertices+
     ' vertices · '+result.triangles+' triangles · SHA-256 '+result.packageSha256);
 });
 return <div className="app-shell">
  <header className="topbar">
   <div><div className="brand">Map2Model <strong>Next</strong></div><div className="subbrand">Local-first terrain to 3D print</div></div>
   <div className="status-pills"><span className="status active">● Offline mode</span><span className="status">{system.engine?.syntheticExportAvailable?'Synthetic 3MF engine ready':'Synthetic engine not installed'}</span></div>
  </header>
  <div className="app-grid">
   <aside className="sidebar">
    <h2>Projects</h2>
    <div className="small-desc">Projects and geographic selections are saved in your selected local library.</div>
    <div className="library-card">
     <strong>Library</strong><p>{system.libraryConfigured?system.libraryLabel:'Not selected'}</p>
     <button type="button" onClick={choose} disabled={busy||!api.chooseLibrary}>{system.libraryConfigured?'Change folder':'Choose library folder'}</button>
    </div>
    <label className="field">Project name<input maxLength={100} value={projectName} onChange={e=>setProjectName(e.target.value)} placeholder="New terrain project"/></label>
    <div className="button-row"><button className="primary" onClick={create} disabled={!system.libraryConfigured||busy}>New project</button><button onClick={save} disabled={!project||busy}>Save</button></div>
    <div className="project-list" role="list" aria-label="Local projects">
     {projects.length===0?<p className="muted">No projects saved yet.</p>:projects.map(item=><button role="listitem" className={item.projectId===project?.projectId?'project-item chosen':'project-item'} key={item.projectId} onClick={()=>open(item.projectId)}>{item.title}<small>{item.selection?'Selection saved':'No selection yet'}</small></button>)}
    </div>
    <div className="sidebar-foot"><strong>Data library</strong><p>No terrain, basemap, buildings, or roads are installed.</p><button disabled title="Region import is planned for the geodata engine">Import region — later milestone</button></div>
   </aside>
   <main className="workspace">
    <div className="workspace-title"><div><h1>{project?.title||'Geographic workspace'}</h1><p>Explore using coordinates or draw a local selection. Maximum 2 km per side in this milestone.</p></div><span className="chip">Earth · EPSG:4326</span></div>
    <MapPane selection={selection} coverage={experimentalSource} projectId={project?.projectId} onSelection={pick} onError={setMessage}/>
    {message?<div className="message" role="status">{message}</div>:null}
   </main>
   <aside className="inspector">
    <h2>Selection & print setup</h2>
    {selection?<div className="detail-card">
      <span className="kicker">Validated geographic selection</span>
      <div className="mono">{selection.bbox.map(v=>v.toFixed(6)).join(', ')}</div>
      <div className="metric">Requested sample spacing <strong>{selection.terrainProfile.targetGroundSampleMeters} m</strong></div>
      <div className="metric">Coordinate system <strong>EPSG:4326</strong></div>
      <div className="metric">Selection identity <code>{selection.selectionId.slice(0,18)}…</code></div>
     </div>:<div className="empty-detail">Draw a rectangle on the map to create a validated Earth selection.</div>}
    <h3>Offline data readiness</h3>
    <div className="readiness"><span>Map basemap</span><strong>Not installed</strong></div>
    <div className="readiness"><span>Mapterhorn terrain</span><strong>Missing</strong></div>
    <div className="readiness"><span>Synthetic terrain engine</span><strong>{system.engine?.syntheticExportAvailable?'Ready':'Unavailable'}</strong></div>
    <div className="readiness"><span>Synthetic 3MF exporter</span><strong>{system.engine?.syntheticExportAvailable?'Ready':'Unavailable'}</strong></div>
    <div className="notice">Choose the LOCAL PMTiles file FIRST to view its approximate blue coverage outline. Draw a green rectangle inside the outline; actual indexed tiles are checked separately. Partial/outside coverage disables experimental export. No source rights, datum or height accuracy are certified. Two fixed synthetic export demos remain unchanged.</div>
    <button onClick={chooseExperimental} disabled={busy||!system.engine?.experimentalLocalAvailable}>Choose private local PMTiles (experimental)</button>
    {experimentalSource?<div className="detail-card" role="group" aria-label="Experimental source consent">
      <p><strong>EXPERIMENTAL — UNVERIFIED · PRIVATE LOCAL USE</strong></p>
      <p><strong>File:</strong> {experimentalSource.fileName}</p>
      <p className="mono">Source SHA-256: {experimentalSource.sourceArchiveSha256}</p>
      <p>Approximate header bounds: {experimentalSource.declaredCoverageBboxWgs84?.map(x=>x.toFixed(5)).join(', ')}</p>
      <p>Source zoom {experimentalSource.zoom} (advertised {experimentalSource.headerMinZoom}–{experimentalSource.headerMaxZoom}); {experimentalSource.indexedTileCount} indexed tiles.</p>
      <p><strong>Indexed selection: {coverageStatus?.status||'draw a rectangle'}</strong></p>
      <p role="status">{coverageMessage(coverageStatus,true,!!selection)}</p>
      <p>Blue dashed line is a HEADER envelope; even indexed-inside can contain invalid pixels.</p>
      <button type="button" onClick={clearExperimental} disabled={busy}>Clear local source</button>
      <p>Vertical datum and licensing: UNKNOWN. Archive attribution: {experimentalSource.sourceAttributionFromArchive||'Not supplied'}</p>
      <label><input type="checkbox" checked={experimentalAccepted} disabled={!coverageAdmitsExperiment(coverageStatus,selection,experimentalSource)} onChange={e=>setExperimentalAccepted(e.target.checked)}/>
        I acquired this source under applicable terms, accept unverified elevations for this job only, and will not redistribute without rights review.
      </label>
      <button onClick={exportExperimental} disabled={busy||!experimentalAccepted||!coverageAdmitsExperiment(coverageStatus,selection,experimentalSource)}>Export PRIVATE experimental 3MF</button>
    </div>:null}
    <button onClick={exportOriginalPmtiles}
      disabled={busy||!system.engine?.originalPmtilesExportAvailable}
      className="generate">{busy?'Working…':'Export ORIGINAL SYNTHETIC PMTiles terrain 3MF'}</button>
    <button onClick={exportSample} disabled={busy||!system.engine?.syntheticExportAvailable}
      className="generate">{busy?'Exporting synthetic 3MF…':'Export SYNTHETIC demo 3MF'}</button>
   </aside>
  </div>
  <footer className="footbar"><span>Map2Model Next · Local PMTiles Coverage Preview 0.0.6</span><span>Network disabled by default · No account or external services</span></footer>
 </div>;
}
