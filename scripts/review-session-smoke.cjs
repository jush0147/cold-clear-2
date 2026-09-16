'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const {performance} = require('node:perf_hooks');
const {ReviewSession} = require('../pkg-node/cold_clear_2.js');
const f = JSON.parse(fs.readFileSync('tests/fixtures/replay-v19.json','utf8'));
const cases = f.cases.filter(c=>c.start.hold).slice(0,8);
const results=[];
for(const c of cases){
  const start={...c.start,board:c.rows.map(r=>[...r].map(v=>v==='.'?null:v))};
  const request={start,incoming:[],pieces_placed:30,garbage_sent:0,
    actual:{placement:c.placement,use_hold:false},depth:4,beam_width:4};
  const run=()=>{
    const s=new ReviewSession(JSON.stringify(request));
    assert.equal(JSON.parse(s.report_json()).comparison,null);
    const begin=performance.now();while(!s.step()){}
    const report=JSON.parse(s.report_json());
    assert.equal(report.status,'complete');assert.equal(report.gravity,0);
    assert.equal(report.processed,report.total_actions);
    assert(report.candidates.every(x=>x.depth===4&&x.beam_width===4));
    const actual=JSON.parse(s.candidate_json(report.comparison.actual_action_id));
    const best=JSON.parse(s.candidate_json(report.comparison.preferred_action_id));
    assert.deepEqual(actual.lines[0].steps[0].packets,c.expected.raw_attack);
    assert.equal(actual.lines[0].steps[0].lines,c.expected.lines);
    const elapsed=performance.now()-begin;s.free();
    return {report,actual,best,elapsed};
  };
  const a=run();const b=run();assert.deepEqual(a.report,b.report);
  results.push({fixture:c.index,elapsed_ms:a.elapsed,actions:a.report.total_actions,
    nodes:a.report.candidates.reduce((n,c)=>n+c.evaluated_transitions,0),comparison:a.report.comparison,
    actual_first:a.actual.lines[0].steps[0],best_first:a.best.lines[0].steps[0]});
}
const c=cases[0];
const start={...c.start,board:c.rows.map(r=>[...r].map(v=>v==='.'?null:v))};
const r={start,incoming:[{lines:8,active:true}],pieces_placed:30,garbage_sent:0,
  actual:{placement:c.placement,use_hold:false},depth:3,beam_width:2};
const s=new ReviewSession(JSON.stringify(r));const begin=performance.now();while(!s.step()){}
const pressure=JSON.parse(s.report_json());assert(pressure.information_set_consistent);
assert.equal(pressure.scenarios,10);const elapsed=performance.now()-begin;s.free();
console.log(JSON.stringify({status:'passed',runtime:'Node WASM',recorded_positions:results,
  pressure:{actions:pressure.total_actions,elapsed_ms:elapsed,comparison:pressure.comparison},
  strength_improvement_proven:false},null,2));
