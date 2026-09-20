// Reference dedicated Worker for the snapshot-v2 artifact. No Tetrp engine/replay.
import init,{analyze_snapshot_json,snapshot_capabilities_json} from './pkg/cold_clear_2.js';
let initialized;
let generation=0;
self.onmessage=async({data})=>{
  const token=++generation;
  const id=data?.id;
  if(data?.type==='dispose'){self.close();return;}
  try {
    if(data?.type!=='analyze')throw new Error('Expected analyze request');
    initialized??=init({module_or_path:new URL('./pkg/cold_clear_2_bg.wasm',import.meta.url)});
    await initialized;
    if(token!==generation)return;
    const result=JSON.parse(analyze_snapshot_json(JSON.stringify(data.request)));
    if(token===generation)self.postMessage({id,result,capabilities:JSON.parse(snapshot_capabilities_json())});
  }catch(error){
    if(token===generation)self.postMessage({id,error:String(error?.message??error)});
  }
};
// Synchronous WASM cannot service cancel messages while searching. The owner
// MUST terminate this Worker to cancel/exit; never fall back to UI-thread WASM.
