'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {performance}=require('node:perf_hooks');
const {analyze_pending_json}=require('../pkg-node/cold_clear_2.js');
const fixture=JSON.parse(fs.readFileSync(path.join(__dirname,'../tests/fixtures/replay-v19.json'),'utf8'));
const results=[];
for(const c of fixture.cases.filter(c=>c.start.hold).slice(0,4)){
 const start={...c.start,board:c.rows.map(r=>[...r].map(v=>v==='.'?null:v))};
 for(const incoming of [[],[{lines:8,active:true}]]){
  const request={start,incoming,pieces_placed:30,garbage_sent:0,iterations:100};
  const run=()=>JSON.parse(analyze_pending_json(JSON.stringify(request)));
  const t=performance.now();const a=run();
  assert.equal(a.gravity,0);assert.equal(a.execution_costs,false);assert.equal(a.frames_per_piece,null);
  assert.equal(a.activation_model,'snapshot');assert.equal(a.scenarios,incoming.length?10:1);
  for(let i=0;i<3;i++)assert.deepEqual(run(),a,`repeatability case ${c.index}`);
  assert.equal(new Set(a.candidates.map(x=>JSON.stringify(x.placement))).size,a.candidates.length);
  results.push({fixture:c.index,incoming:incoming.length?'active':'empty',repetitions:4,nodes:a.nodes,scenarios:a.scenarios,elapsed_ms:performance.now()-t,top:a.candidates[0]});
 }
}
console.log(JSON.stringify({status:'passed',deterministic_review:true,gravity:0,execution_costs:false,results},null,2));
