'use strict';
const {contextBridge,ipcRenderer}=require('electron');
const allowed=new Set(['state','chooseLibrary','listProjects','createProject','loadProject','saveProject','engineHealth']);
const invoke=(operation,payload={})=>{
  if(!allowed.has(operation))return Promise.reject(new Error('UNSUPPORTED_OPERATION'));
  return ipcRenderer.invoke('m2m:'+operation,payload);
};
contextBridge.exposeInMainWorld('map2model',{
  getState:()=>invoke('state'),
  chooseLibrary:()=>invoke('chooseLibrary'),
  listProjects:()=>invoke('listProjects'),
  createProject:title=>invoke('createProject',{title}),
  loadProject:projectId=>invoke('loadProject',{projectId}),
  saveProject:project=>invoke('saveProject',project),
  engineHealth:()=>invoke('engineHealth')
});
