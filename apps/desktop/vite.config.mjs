import {defineConfig} from 'vite';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const directory=path.dirname(fileURLToPath(import.meta.url));
export default defineConfig({
 root:directory,
 base:'./',
 server:{host:'127.0.0.1',port:5173,strictPort:true,fs:{allow:[path.resolve(directory,'../..')]}},
 build:{outDir:path.join(directory,'dist'),emptyOutDir:true,sourcemap:false,target:'chrome120'}
});
