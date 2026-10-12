import {randomBytes} from 'node:crypto';
import {mkdir,open,readFile,rename,readdir,rm,lstat} from 'node:fs/promises';
import path from 'node:path';
import {assertSelection} from './selection.mjs';

export const PROJECT_FORMAT_VERSION=1;
const MAX_PROJECT_BYTES=64*1024;
const ID_PATTERN=/^proj-[0-9a-f]{32}$/;

export function normalizeTitle(title){
  if(typeof title!=='string')throw Error('INVALID_PROJECT: title must be text');
  const clean=title.trim();
  if(clean.length<1||clean.length>100||/[\u0000-\u001f\u007f]/u.test(clean))
    throw Error('INVALID_PROJECT: title must be 1–100 printable characters');
  return clean;
}
function validId(id){
  if(typeof id!=='string'||!ID_PATTERN.test(id))throw Error('INVALID_PROJECT: invalid project identity');
  return id;
}
export function assertProject(project){
  if(!project||typeof project!=='object'||Array.isArray(project)||
     Object.keys(project).some(k=>!['formatVersion','projectId','title','selection','createdAt','updatedAt'].includes(k))||
     project.formatVersion!==PROJECT_FORMAT_VERSION)
    throw Error('INVALID_PROJECT: unsupported version or fields');
  validId(project.projectId);
  normalizeTitle(project.title);
  if(project.selection!==null)assertSelection(project.selection);
  for(const key of ['createdAt','updatedAt']){
    if(typeof project[key]!=='string'||!/^\d{4}-\d\d-\d\dT/.test(project[key])||
       !Number.isFinite(Date.parse(project[key])))throw Error('INVALID_PROJECT: timestamp is invalid');
  }
  return project;
}
async function projectDir(root){
  if(typeof root!=='string'||!path.isAbsolute(root))throw Error('INVALID_LIBRARY: an approved absolute library is required');
  const directory=path.join(root,'projects');
  await mkdir(directory,{recursive:true,mode:0o700});
  return directory;
}
async function atomicJson(filename,body){
  const bytes=Buffer.from(JSON.stringify(body,null,2)+'\n','utf8');
  if(bytes.length>MAX_PROJECT_BYTES)throw Error('INVALID_PROJECT: project exceeds 64 KiB');
  const staging=filename+'.'+randomBytes(8).toString('hex')+'.tmp';
  const fd=await open(staging,'wx',0o600);
  try{
    await fd.writeFile(bytes);await fd.sync();
  }finally{await fd.close();}
  try{await rename(staging,filename);}
  catch(e){await rm(staging,{force:true});throw e;}
}
async function checkedRead(filename){
  const st=await lstat(filename); // Reject links, folders and unexpectedly large content.
  if(!st.isFile()||st.isSymbolicLink()||st.size>MAX_PROJECT_BYTES)
    throw Error('INVALID_PROJECT: unsafe project file');
  const data=JSON.parse(await readFile(filename,'utf8'));
  return assertProject(data);
}
export async function createProject(root,title='Untitled project'){
  const directory=await projectDir(root),now=new Date().toISOString();
  const value={formatVersion:PROJECT_FORMAT_VERSION,projectId:'proj-'+randomBytes(16).toString('hex'),
    title:normalizeTitle(title),selection:null,createdAt:now,updatedAt:now};
  const file=path.join(directory,value.projectId+'.json');
  await atomicJson(file,value);
  return value;
}
export async function loadProject(root,id){
  const directory=await projectDir(root);
  return checkedRead(path.join(directory,validId(id)+'.json'));
}
export async function saveProject(root,payload){
  if(!payload||typeof payload!=='object')throw Error('INVALID_PROJECT: missing save payload');
  const id=validId(payload.projectId),existing=await loadProject(root,id);
  const next={...existing,title:normalizeTitle(payload.title),selection:payload.selection===null?null:assertSelection(payload.selection),
    updatedAt:new Date().toISOString()};
  await atomicJson(path.join(await projectDir(root),id+'.json'),next);
  return next;
}
export async function listProjects(root){
  const directory=await projectDir(root),names=await readdir(directory);
  const projects=[];
  for(const name of names.filter(x=>/^proj-[0-9a-f]{32}\.json$/.test(x)).slice(0,1000)){
    try{const record=await checkedRead(path.join(directory,name));projects.push(record);}
    catch{/* Corrupt documents cannot elevate privileges or prevent opening valid projects. */}
  }
  return projects.sort((a,b)=>b.updatedAt.localeCompare(a.updatedAt));
}
