import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createServer} from 'node:http';
import {createRequire} from 'node:module';

const artifactRoot=path.resolve(process.argv[2]||'kiwi-v1-browser');
const tetrpRoot=path.resolve(process.argv[3]||'tetrp-reference');
const require=createRequire(path.join(tetrpRoot,'package.json'));
const {chromium}=require('playwright');

const roots={artifact:artifactRoot,tetrp:tetrpRoot};
const contentType=file=>file.endsWith('.wasm')?'application/wasm':
  file.endsWith('.json')?'application/json':
  file.endsWith('.js')||file.endsWith('.mjs')?'text/javascript':'application/octet-stream';
const server=createServer(async(req,res)=>{
  try{
    const u=new URL(req.url,'http://localhost');
    if(u.pathname==='/'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Kiwi packaged E2E</title>');return;}
    const match=/^\/(artifact|tetrp)\/(.+)$/.exec(decodeURIComponent(u.pathname));
    if(!match){res.writeHead(404).end();return;}
    const root=roots[match[1]],file=path.resolve(root,match[2]);
    if(file!==root&&!file.startsWith(root+path.sep)){res.writeHead(403).end();return;}
    const bytes=await fs.readFile(file);
    res.setHeader('Content-Type',contentType(file));res.end(bytes);
  }catch{res.writeHead(404).end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base='http://127.0.0.1:'+server.address().port+'/';

const browser=await chromium.launch({headless:true});
try{
  const page=await browser.newPage();
  const requests=[];
  page.on('request',r=>requests.push(r.url()));
  await page.goto(base);
  const result=await page.evaluate(async()=>{
    // These imports are deliberately FROM THE PACKAGED ARTIFACT. A missing helper
    // dependency therefore fails this post-package test before any search starts.
    const [{createPlacementTools},{captureSnapshotFromEngine,buildSnapshotRequest},engineMod,B,R,wasm]=await Promise.all([
      import('/artifact/tetrp-placement-path.mjs'),
      import('/artifact/kiwi-snapshot-adapter.mjs'),
      import('/tetrp/src/engine.js'),
      import('/tetrp/src/board.js'),
      import('/tetrp/src/rotation.js'),
      import('/artifact/pkg/cold_clear_2.js'),
    ]);
    const {Engine}=engineMod;
    const tools=createPlacementTools({Engine,boardModule:B,rotationModule:R});
    await wasm.default({module_or_path:new URL('/artifact/pkg/cold_clear_2_bg.wasm',location.origin)});
    const handling={arr:0,das:1,dcd:0,sdf:20,safelock:false,cancel:false,may20g:true,irs:'off',ihs:'off'};

    const tl=new Engine({
      mode:'tl',seed:123,
      rules:{g:0,gincrease:0,b2bcharge_base:3,garbageare:5,garbagearebump:12},
      handling
    });
    const tlRequest=buildSnapshotRequest(captureSnapshotFromEngine(tl,tools),{nodeBudget:5000,framesPerPiece:24});
    const tlResult=JSON.parse(wasm.analyze_snapshot_json(JSON.stringify(tlRequest)));

    const stacking=new Engine({mode:'40l',seed:456,rules:{g:0,garbageare:5,garbagearebump:12},handling});
    const stackingRequest=buildSnapshotRequest(captureSnapshotFromEngine(stacking,tools),{nodeBudget:5000,framesPerPiece:24});
    const stackingResult=JSON.parse(wasm.analyze_snapshot_json(JSON.stringify(stackingRequest)));

    return {
      tl:{
        request_mode:[tlRequest.analysis_mode,tlRequest.source_mode],
        timing_rules:tlRequest.timing_rules,
        result_mode:[tlResult.analysis_mode,tlResult.source_mode],
        nodes:tlResult.nodes,budget:tlResult.node_budget,completion:tlResult.completion,
        authority_attack_clock:tlResult.authority_attack_clock,
      },
      stacking:{
        request_mode:[stackingRequest.analysis_mode,stackingRequest.source_mode],
        neutral:{
          combo:stackingRequest.start.combo,b2b:stackingRequest.start.back_to_back,
          b2b_count:stackingRequest.start.b2b_count,incoming:stackingRequest.incoming.length,
        },
        attack_clock:[
          stackingRequest.authority_frame,stackingRequest.garbage_multiplier,
          stackingRequest.garbage_margin_frames,stackingRequest.garbage_increase_per_second
        ],
        result_mode:[stackingResult.analysis_mode,stackingResult.source_mode],
        nodes:stackingResult.nodes,budget:stackingResult.node_budget,completion:stackingResult.completion,
        authority_attack_clock:stackingResult.authority_attack_clock,
      }
    };
  });

  assert.deepEqual(result.tl.request_mode,['tl','tl']);
  assert.deepEqual(result.tl.result_mode,['tl','tl']);
  assert.equal(result.tl.timing_rules.garbage_are_frames,5);
  assert.equal(result.tl.timing_rules.garbage_are_bump_frames,12);
  assert.equal(result.tl.authority_attack_clock,true);
  assert.ok(result.tl.nodes<=result.tl.budget);

  assert.deepEqual(result.stacking.request_mode,['competitive_stacking','40l']);
  assert.deepEqual(result.stacking.result_mode,['competitive_stacking','40l']);
  assert.deepEqual(result.stacking.neutral,{combo:0,b2b:false,b2b_count:0,incoming:0});
  assert.deepEqual(result.stacking.attack_clock,[null,null,null,null]);
  assert.equal(result.stacking.authority_attack_clock,false);
  assert.ok(result.stacking.nodes<=result.stacking.budget);
  assert.ok(requests.every(u=>u.startsWith(base)),'packaged E2E must remain local-only');

  console.log(JSON.stringify({
    status:'passed',
    schema:'kiwi-packaged-e2e/1',
    evidence:'import packaged placement helper -> pinned Tetrp snapshot -> packaged web WASM search',
    browser:{name:'Chromium',version:browser.version()},
    local_only:true,
    ...result
  },null,2));
}finally{
  await browser.close();
  await new Promise(resolve=>server.close(resolve));
}
