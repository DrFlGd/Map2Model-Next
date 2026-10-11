'use strict';
const {app,BrowserWindow,dialog,ipcMain,session}=require('electron');
const path=require('node:path');
const fs=require('node:fs/promises');

const DEV=process.env.MAP2MODEL_DEV==='1'&&!app.isPackaged;
const DEV_ORIGIN='http://127.0.0.1:5173';
const USERDATA_PREFS='desktop-prefs-v1.json';
let window=null;
let prefs={library:null};
let projects=null;
let mainOrigin='file:';

function plainFailure(error){
  const text=String(error?.message||'INTERNAL_ERROR');
  if(/^([A-Z_]+):/.test(text))return text.slice(0,240);
  return 'INTERNAL_ERROR: operation failed; inspect project and library permissions';
}
async function state(){
  return {appVersion:app.getVersion(),offline:true,networkEnabled:false,
    libraryLabel:prefs.library?path.basename(prefs.library):null,
    libraryConfigured:Boolean(prefs.library),
    engine:{status:'not-installed',canGenerate:false},
    basemap:{status:'not-installed'},terrain:{status:'not-installed'}};
}
async function prefsPath(){return path.join(app.getPath('userData'),USERDATA_PREFS);}
async function loadPrefs(){
  try{
    const data=JSON.parse(await fs.readFile(await prefsPath(),'utf8'));
    if(data&&typeof data.library==='string'&&path.isAbsolute(data.library))
      prefs.library=data.library;
  }catch{/* First run has no preferences. */}
}
async function savePrefs(){
  const destination=await prefsPath();
  const staging=destination+'.tmp';
  await fs.writeFile(staging,JSON.stringify(prefs,null,2)+'\n',{mode:0o600});
  await fs.rename(staging,destination);
}
async function ensureLibrary(){
  if(!prefs.library)throw Error('LIBRARY_MISSING: choose a local library folder first');
  const stat=await fs.stat(prefs.library);
  if(!stat.isDirectory())throw Error('LIBRARY_MISSING: selected library folder is unavailable');
  return prefs.library;
}
function validateCaller(event){
  if(!window||event.sender!==window.webContents||
    !event.senderFrame||!event.senderFrame.url.startsWith(mainOrigin))
    throw Error('INVALID_REQUEST: IPC source not trusted');
}
function handler(name,fn){
  ipcMain.handle('m2m:'+name,async(event,payload)=>{
    validateCaller(event);
    if(payload&&JSON.stringify(payload).length>65536)
      throw Error('INVALID_REQUEST: IPC payload too large');
    try{return await fn(payload||{});}catch(error){throw Error(plainFailure(error));}
  });
}
async function installHandlers(){
  projects=await import('../shared/projects.mjs');
  handler('state',async()=>state());
  handler('chooseLibrary',async()=>{
    const answer=await dialog.showOpenDialog(window,{title:'Choose an offline project library',
      properties:['openDirectory','createDirectory']});
    if(answer.canceled||!answer.filePaths[0])return state();
    const selected=await fs.realpath(answer.filePaths[0]);
    const st=await fs.stat(selected);
    if(!st.isDirectory())throw Error('INVALID_LIBRARY: selected folder is not a directory');
    await fs.mkdir(path.join(selected,'projects'),{recursive:true});
    prefs.library=selected;await savePrefs();
    return state();
  });
  handler('listProjects',async()=>projects.listProjects(await ensureLibrary()));
  handler('createProject',async p=>projects.createProject(await ensureLibrary(),p.title||'New terrain project'));
  handler('loadProject',async p=>projects.loadProject(await ensureLibrary(),p.projectId));
  handler('saveProject',async p=>projects.saveProject(await ensureLibrary(),p));
  handler('engineHealth',async()=>({status:'not-installed',canGenerate:false,
    reason:'Native terrain and 3MF engines are pending A2-01/A3-01 integration'}));
}
function approvedUrl(url){
  if(typeof url!=='string')return false;
  if(DEV)return url.startsWith(DEV_ORIGIN+'/')||url===DEV_ORIGIN;
  return url.startsWith(mainOrigin);
}
async function main(){
  await loadPrefs();
  const index=path.join(__dirname,'../dist/index.html');
  const fileUrl=require('node:url').pathToFileURL(index).toString();
  mainOrigin=DEV?DEV_ORIGIN+'/':fileUrl;
  const localSession=session.defaultSession;
  // Deliberate network-block mode. Even the map style is generated locally.
  localSession.webRequest.onBeforeRequest({urls:['*://*/*']},(details,callback)=>{
    callback({cancel:!approvedUrl(details.url)});
  });
  await installHandlers();
  window=new BrowserWindow({
    width:1400,height:900,minWidth:980,minHeight:680,
    title:'Map2Model Next — Offline Workspace',show:false,backgroundColor:'#101a22',
    webPreferences:{
      preload:path.join(__dirname,'preload.cjs'),
      sandbox:true,contextIsolation:true,nodeIntegration:false,
      webSecurity:true,webviewTag:false,
      allowRunningInsecureContent:false
    }
  });
  window.webContents.setWindowOpenHandler(()=>({action:'deny'}));
  window.webContents.on('will-navigate',(event,url)=>{
    if(!approvedUrl(url))event.preventDefault();
  });
  window.once('ready-to-show',()=>window?.show());
  if(DEV)await window.loadURL(DEV_ORIGIN);
  else await window.loadFile(index);
}
app.whenReady().then(main).catch(err=>{
  console.error('Desktop initialization failed:',err?.message||err);
  app.quit();
});
app.on('window-all-closed',()=>{if(process.platform!=='darwin')app.quit();});
app.on('activate',()=>{if(BrowserWindow.getAllWindows().length===0)main().catch(()=>app.quit());});
