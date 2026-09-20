import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createServer} from 'node:http';
import {createRequire} from 'node:module';
const require=createRequire(path.resolve('tetrp-reference/package.json'));
const {chromium,webkit}=require('playwright');
const root=path.resolve('kiwi-v1-browser');
const request=JSON.parse(await fs.readFile('snapshot-browser-request.json','utf8'));
const server=createServer(async(req,res)=>{
  try{
    const name=decodeURIComponent(new URL(req.url,'http://localhost').pathname).slice(1);
    if(!name){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Kiwi Worker acceptance</title>');return;}
    const file=path.resolve(root,name);
    if(!file.startsWith(root+path.sep)){res.writeHead(403).end();return;}
    const bytes=await fs.readFile(file);
    res.setHeader('Content-Type',name.endsWith('.wasm')?'application/wasm':name.endsWith('.js')||name.endsWith('.mjs')?'text/javascript':'application/json');
    res.end(bytes);
  }catch{res.writeHead(404).end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base='http://127.0.0.1:'+server.address().port+'/';
const results=[];
try{
  for(const [name,kind] of [['chromium',chromium],['webkit',webkit]]){
    const browser=await kind.launch({headless:true});
    try{
      const page=await browser.newPage();const urls=[];
      page.on('request',r=>urls.push(r.url()));
      await page.goto(base);
      const result=await page.evaluate(async request=>{
        const create=()=>new Worker('./kiwi-snapshot-worker.mjs',{type:'module'});
        let worker=create(),ticks=0;
        const timer=setInterval(()=>ticks++,5);
        const run=(id,r)=>new Promise((resolve,reject)=>{
          const timeout=setTimeout(()=>reject(new Error('Worker timed out')),60000);
          worker.onmessage=({data})=>{if(data.id!==id)return;clearTimeout(timeout);data.error?reject(Object.assign(new Error(data.error.message??String(data.error)),{code:data.error.code??'WORKER_ERROR'})):resolve({result:data.result,capabilities:data.capabilities});};
          worker.onerror=e=>{clearTimeout(timeout);reject(new Error(e.message));};
          worker.postMessage({type:'analyze',id,request:r});
        });
        try{
          const a=await run(1,request);
          const b=await run(2,request);
          const identical=JSON.stringify(a)===JSON.stringify(b);
          const unknownRequest={...request,incoming:[{lines:4,ready_in_frames:null}]};
          const unknownStart=performance.now();
          const unknown=await run(10,unknownRequest);
          const unknownMs=performance.now()-unknownStart;
          const unknownRepeated=await run(11,unknownRequest);
          let rejectionCode=null;
          try{await run(20,{...request,history:[]});}catch(error){rejectionCode=error.code;}
          let lateReplies=0;
          worker.onmessage=()=>lateReplies++;
          worker.postMessage({type:'analyze',id:3,request:{...request,node_budget:2000000}});
          await new Promise(resolve=>setTimeout(resolve,15));
          worker.terminate();
          await new Promise(resolve=>setTimeout(resolve,100));
          worker=create();
          const c=await run(4,request);
          return {
            identical,restarted_identical:JSON.stringify(a)===JSON.stringify(c),
            rejectionCode,ticks,lateReplies,
            nodes:a.result.nodes,budget:a.result.node_budget,action_kind:a.result.action.kind,
            same_piece_hold_search:a.capabilities.same_piece_hold_search,
            root_geometry_in_search:a.capabilities.root_geometry_in_search,
            unknown_activation:{
              scenarios:unknown.result.scenarios,packets:unknown.result.unknown_activation_packets,
              delays:unknown.result.unknown_activation_delays,nodes:unknown.result.nodes,
              budget:unknown.result.node_budget,ms:unknownMs,
              deterministic:JSON.stringify(unknown)===JSON.stringify(unknownRepeated)
            }
          };
        }finally{clearInterval(timer);worker.terminate();}
      },request);
      assert.equal(result.identical,true);assert.equal(result.restarted_identical,true);
      assert.equal(result.lateReplies,0);assert.ok(result.ticks>0);assert.ok(result.nodes<=200000);
      assert.equal(result.rejectionCode,'REQUEST_SCHEMA_INVALID');
      assert.equal(result.same_piece_hold_search,true);
      assert.equal(result.root_geometry_in_search,true);
      assert.equal(result.unknown_activation.scenarios,30);
      assert.equal(result.unknown_activation.packets,1);
      assert.deepEqual(result.unknown_activation.delays,[1,25,600]);
      assert.ok(result.unknown_activation.nodes<=200000);
      assert.equal(result.unknown_activation.deterministic,true);
      assert.ok(urls.every(u=>u.startsWith(base)),'browser must not contact a remote bot service');
      results.push({browser:name,version:browser.version(),...result});
    }finally{await browser.close();}
  }
}finally{await new Promise(resolve=>server.close(resolve));}
console.log(JSON.stringify({status:'passed',schema:'kiwi-snapshot-browser/3',results,
  evidence:'real headless browser module Workers using the delivered web WASM; not physical-device certification'},null,2));
