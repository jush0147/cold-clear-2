import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {
  KiwiSnapshotError,captureSnapshot,captureSnapshotFromEngine,buildSnapshotRequest,
  normalizeSnapshotError,selectReachableSnapshotAction,validateSnapshotAction,
  validateSnapshotTimingAction,applyHoldForReanalysis,assertSupportedSnapshotRules,snapshotRuleContract
} from './lib/kiwi-snapshot-adapter.mjs';
import {createPlacementTools} from './lib/tetrp-placement-path.mjs';

const require=createRequire(import.meta.url);
const wasm=require('../pkg-node/cold_clear_2.js');
const root=path.resolve(process.argv[2]||'tetrp-reference');
const load=p=>import(pathToFileURL(path.join(root,'src',p)).href);
const {Engine}=await load('engine.js');
const B=await load('board.js'),R=await load('rotation.js');
const {createBag,pullBag}=await load('random.js');
const tools=createPlacementTools({Engine,boardModule:B,rotationModule:R});
const deps={Engine,placementTools:tools};
const handling={arr:0,das:1,dcd:0,sdf:20,safelock:false,cancel:false,may20g:true,irs:'off',ihs:'off'};
const make=(seed=1,rules={})=>new Engine({mode:'tl',seed,rules:{g:0,gincrease:0,b2bcharge_base:3,...rules},handling});
const make40l=(seed=1,rules={})=>new Engine({mode:'40l',seed,rules:{g:0,...rules},handling});
const request=(e,nodes=5000)=>buildSnapshotRequest(captureSnapshotFromEngine(e,tools),{nodeBudget:nodes,framesPerPiece:24});
const analyze=r=>JSON.parse(wasm.analyze_snapshot_json(JSON.stringify(r)));
const codeOf=fn=>{try{fn();assert.fail('expected rejection');}catch(e){return normalizeSnapshotError(e).code;}};
const checks=[];
const caps=JSON.parse(wasm.snapshot_capabilities_json());
assert.equal(caps.schema,'kiwi-snapshot-capabilities/3');
assert.equal(caps.same_piece_hold_search,true);
assert.equal(caps.hold_information_gain_optimized,false);
assert.equal(caps.root_geometry_in_search,true);
assert.equal(caps.root_timing_in_search,false);
assert.deepEqual(caps.supported_source_modes,['tl','40l']);
assert.equal(caps.competitive_stacking_mode,true);
assert.equal(caps.garbage_are_rule_transport,true);
assert.equal(caps.garbage_are_bump_rule_transport,true);
assert.equal(caps.exact_are_bump_timing,false);

// Pure current-snapshot boundary: geometry and request construction must not
// inspect hidden queue tail, RNG, history or opponent state.
{
  const e=make(42),s=e.state;
  s.bag.queue=new Proxy(s.bag.queue,{get(t,k,r){
    if(/^\d+$/.test(String(k))&&Number(k)>=5)throw new Error('HIDDEN_NEXT_READ');
    return Reflect.get(t,k,r);
  }});
  Object.defineProperty(s.bag,'rng',{get(){throw new Error('HIDDEN_RNG_READ')}});
  Object.defineProperty(s,'observedDraws',{get(){throw new Error('HISTORY_READ')}});
  Object.defineProperty(s,'opponent',{get(){throw new Error('OPPONENT_READ')}});
  const v=captureSnapshotFromEngine(e,tools);
  const a=buildSnapshotRequest(v,{nodeBudget:5000});
  assert.equal(a.start.queue.length,6);
  assert.ok(!JSON.stringify(a).includes('observedDraws'));
  assert.ok(!JSON.stringify(a).includes('randomizer'));
  assert.ok(!JSON.stringify(a).includes('bag_state'));
  analyze(a);
}
checks.push('root geometry + request projection read only current visible state, never hidden NEXT/RNG/history/opponent');

// Same visible snapshot with different hidden tails yields byte-identical request
// and result. Use plain state holders so no serializer can accidentally save us.
{
  const a=make(10),b=make(11);
  b.state=structuredClone(a.state);
  const visible=a.state.bag.queue.slice(0,5);
  b.state.bag.queue=[...visible,'z','z','z','z','z'];
  b.state.bag.rng={seed:999};
  const ra=request(a),rb=request(b);
  assert.deepEqual(ra,rb);
  assert.deepEqual(analyze(ra),analyze(rb));
}
checks.push('hidden tail/RNG invariance');

// Explicit same-piece Hold: empty and occupied branches are separate actions and
// never carry a landing. Empty pre-reveal scoring cannot see the new preview.
for(const variant of ['empty','occupied']){
  const e=make(20+(variant==='occupied'));
  const current=e.state.piece.type;
  if(variant==='empty'){
    e.state.hold={piece:null,locked:false};
    e.state.bag.queue[0]=current;
  }else{
    e.state.hold={piece:current,locked:false};
  }
  const r=request(e),result=analyze(r);
  const h=result.candidates.find(c=>c.action.kind==='hold');
  assert.ok(h,variant+' same-piece Hold missing');
  assert.equal(h.action.mode,variant);
  assert.equal(h.action.same_piece,true);
  assert.equal(h.action.requires_reanalysis,true);
  assert.ok(!('placement'in h.action));
  if(variant==='empty')assert.equal(h.search_basis,'post_empty_hold_known_prefix_without_revealed_next');
  else assert.equal(h.search_basis,'post_hold_visible_state');
  validateSnapshotAction(e,h.action,deps);
}
checks.push('empty/occupied same-piece Hold is explicit, landing-free, and empty reveal information gain is not fabricated');

// Actually execute Hold only after a product choice, then reveal/refill and run a
// NEW locked request. This authority-side test is allowed to consume the private
// sequence precisely because it models the post-choice branch.
{
  const e=make(30),current=e.state.piece.type;
  e.state.bag.queue[0]=current;
  const original=e.serialize(),before=e.state.bag.queue.slice(0,6);
  const pre=analyze(request(e));
  const hold=pre.candidates.find(c=>c.action.kind==='hold'&&c.action.same_piece).action;
  const fork=applyHoldForReanalysis(e,hold,{Engine});
  assert.equal(e.serialize(),original);
  const post=request(fork);
  assert.equal(post.hold_locked,true);
  assert.equal(post.start.hold,current.toUpperCase());
  assert.deepEqual(post.start.queue,before.map(x=>x.toUpperCase()));
  const rr=analyze(post);
  assert.ok(rr.candidates.every(c=>c.action.kind==='place'));
  assert.ok(!fork.hold());
}
{
  const e=make(31),current=e.state.piece.type;
  e.state.hold={piece:current,locked:false};
  const original=e.serialize(),q=JSON.stringify(e.state.bag.queue);
  const h=analyze(request(e)).candidates.find(c=>c.action.kind==='hold'&&c.action.same_piece).action;
  const fork=applyHoldForReanalysis(e,h,{Engine});
  assert.equal(e.serialize(),original);
  assert.equal(JSON.stringify(fork.state.bag.queue),q);
  assert.equal(request(fork).hold_locked,true);
  assert.equal(h.mode,'occupied');
}
checks.push('post-Hold reanalysis: empty consumes one draw/refills preview; occupied consumes zero draws; root is locked');

// Geometry fixtures: actual non-spawn positions, wall, rotated, near-lock, and
// an immobile spin-tagged state. Every Place candidate must be in the exhaustive
// authority-derived root allowlist. No top-K fallback exists.
const geometryFixtures=[];
{
  const e=make(40);e.move(-1);e.descend(4);geometryFixtures.push(['non_spawn',e]);
}
{
  const e=make(41);while(e.move(-1)){}geometryFixtures.push(['wall',e]);
}
{
  const e=make(42);assert.ok(e.rotate(1));e.descend(2);geometryFixtures.push(['rotated',e]);
}
{
  const e=make(43);e.slam();geometryFixtures.push(['near_lock',e]);
}
{
  const e=make(44);
  e.state.piece.type='t';e.state.piece.x=4;e.state.piece.y=20;e.state.piece.hy=20;
  e.state.piece.r=0;e.state.piece.rotated=true;e.state.piece.totalRotations=1;
  const occupied=new Set(B.cells(e.state.piece).map(([x,y])=>x+','+Math.ceil(y)));
  for(let y=18;y<=22;y++)for(let x=2;x<=6;x++){
    if(!occupied.has(x+','+y))e.state.board.rows[y][x]='i';
  }
  e.state.piece.spin=R.classifySpin(e.state.board,e.state.piece,e.state.rules.spinbonuses);
  assert.notEqual(e.state.piece.spin,'none');
  geometryFixtures.push(['spin_state',e]);
}
for(const [name,e] of geometryFixtures){
  const geo=tools.enumerateRootPlacements(e);
  assert.ok(geo.states_explored>0,name);
  const req=buildSnapshotRequest(captureSnapshot(e.state,{rootGeometry:geo}),{nodeBudget:5000});
  const result=analyze(req),allowed=new Set(req.root_legal_placements.map(JSON.stringify));
  for(const c of result.candidates){
    if(c.action.kind==='place')assert.ok(allowed.has(JSON.stringify(c.action.placement)),name+' leaked spawn-only candidate');
  }
  const selected=selectReachableSnapshotAction(e,result,deps);
  assert.ok(Number.isInteger(selected.candidate_index));
  assert.equal(result.candidates[selected.candidate_index].candidate_index,selected.candidate_index);
  if(selected.action.kind==='place')assert.equal(selected.geometry_validated,true);
}
checks.push('root geometry fixtures: non-spawn/wall/rotated/near-lock/spin states are filtered inside search and candidate index is reported');

// Geometry and input timing are deliberately separate claims.
{
  const e=make(45);e.slam();e.state.piece.safelock=7;
  const result=analyze(request(e));
  const place=result.candidates.find(c=>c.action.kind==='place');
  assert.ok(place);
  const g=validateSnapshotAction(e,place.action,deps);
  assert.equal(g.geometry_validated,true);assert.equal(g.timing_validated,false);
  assert.equal(codeOf(()=>validateSnapshotTimingAction(e,place.action,{...deps,framesPerPiece:1})),'ROOT_TIMING_UNEXECUTABLE');
}
checks.push('geometry reachability is distinct from frame/reset timing executability');

// Real replay TL timing rules are preserved even though the forecast still
// discloses that exact ARE/bump timing is not simulated.
{
  const e=make(49,{garbageare:5,garbagearebump:12});
  const r=request(e),result=analyze(r);
  assert.equal(r.analysis_mode,'tl');assert.equal(r.source_mode,'tl');
  assert.deepEqual(r.timing_rules,{
    garbage_are_frames:5,garbage_are_bump_frames:12,garbage_locked_until_frame:0
  });
  assert.equal(result.garbage_are_frames,5);
  assert.equal(result.garbage_are_bump_frames,12);
  assert.equal(result.exact_are_bump_timing,false);
  assert.equal(result.authority_attack_clock,true);
}
checks.push('real TL garbageare=5 and garbagearebump=12 are preserved without being zero-filled or overclaimed');

// 40L is a separately labeled competitive stacking heuristic. It has no TL
// attack state, pending garbage or attack clock, and starts from neutral combo/B2B.
{
  const e=make40l(491);
  const r=request(e),result=analyze(r);
  assert.equal(r.source_mode,'40l');
  assert.equal(r.analysis_mode,'competitive_stacking');
  assert.equal(r.start.combo,0);assert.equal(r.start.back_to_back,false);assert.equal(r.start.b2b_count,0);
  assert.deepEqual(r.incoming,[]);
  assert.equal(r.authority_frame,null);assert.equal(r.garbage_multiplier,null);
  assert.equal(r.garbage_margin_frames,null);assert.equal(r.garbage_increase_per_second,null);
  assert.equal(result.source_mode,'40l');
  assert.equal(result.analysis_mode,'competitive_stacking');
  assert.equal(result.authority_attack_clock,false);
  assert.equal(result.competitive_stacking_neutral_root_counters,true);
  assert.equal(result.search_path,'stateless_competitive_stacking_snapshot_split_root_actions');
}
checks.push('40L uses explicit competitive_stacking mode with neutral combo/B2B and no fake TL attack clock');

// Stable rejection contract for pending/ARE and rule states.
{
  const e=make(50);e.state.attack.are=[{amt:2}];
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'PENDING_ARE_QUEUE_UNSUPPORTED');
}
{
  const e=make(51);e.state.attack.pending=[{cid:1,amt:2,active:false,activeFrame:null,hardened:false,shielded:false,status:'spawn'}];
  const unknownRequest=request(e),unknownResult=analyze(unknownRequest);
  assert.equal(unknownRequest.incoming[0].ready_in_frames,null);
  assert.equal(unknownResult.unknown_activation_packets,1);
  assert.equal(unknownResult.scenarios,30);
  assert.ok(unknownResult.nodes<=unknownRequest.node_budget);
  assert.deepEqual(analyze(unknownRequest),unknownResult);
  e.state.attack.pending[0]={...e.state.attack.pending[0],active:true,hardened:true};
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'PENDING_PACKET_HARDENED_UNSUPPORTED');
  e.state.attack.pending[0]={...e.state.attack.pending[0],hardened:false,shielded:true};
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'PENDING_PACKET_SHIELDED_UNSUPPORTED');
  e.state.attack.pending[0]={...e.state.attack.pending[0],shielded:false,status:'other'};
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'PENDING_PACKET_STATUS_UNSUPPORTED');
}
{
  const e=make(52);e.state.rules.garbagecap=9;
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'RULE_VALUE_UNSUPPORTED');
}
{
  const e=make(521);e.state.rules.b2bchaining=true;
  assert.equal(codeOf(()=>captureSnapshotFromEngine(e,tools)),'RULE_VALUE_UNSUPPORTED');
}
{
  const e=make(53);
  Object.assign(e.state.rules,{b2bcharging:false,b2bcharge_at:7,b2bcharge_base:5,openerphase_pieces:20,
    allclears:false,allclear_garbage:7,allclear_b2b:2,garbagespecialbonus:false,clutch:false});
  assert.doesNotThrow(()=>assertSupportedSnapshotRules(e.state.rules));
  assert.doesNotThrow(()=>captureSnapshotFromEngine(e,tools));
}
assert.equal(snapshotRuleContract().rejections.positive_are,'PENDING_ARE_QUEUE_UNSUPPORTED');
checks.push('stable structured rejection codes and supported-vs-exact rule contract');

// Pending that has not entered ARE and late multiplier use the same clocked API.
{
  const e=make(60),cid=e.receive({from:'P2',iid:1,ackiid:0,amt:4});
  const unknown=request(e),unknownResult=analyze(unknown);
  assert.equal(unknown.incoming[0].ready_in_frames,null);
  assert.equal(unknownResult.scenarios,30);
  assert.deepEqual(unknownResult.unknown_activation_delays,[1,25,600]);
  e.confirm(cid);
  const r=request(e),result=analyze(r);
  assert.equal(result.scenarios,10);assert.equal(result.authority_attack_clock,true);
  assert.ok(result.nodes<=r.node_budget);
}
{
  const e=make(61);e.state.attack.multiplier=1.75;
  const r=request(e),result=analyze(r);
  assert.equal(r.incoming.length,0);assert.equal(r.garbage_multiplier,1.75);
  assert.equal(result.search_path,'stateless_clocked_snapshot_split_root_actions');
}
checks.push('pre-ARE pending and no-pending late multiplier share the clock-aware snapshot API');

// Progressive reveal across more than the initial window. The private sequence is
// consumed only by this isolated branch executor, never by Worker input.
{
  const recorded=make(70),recordedBytes=recorded.serialize();
  let branch=Engine.restore(recordedBytes);
  const model=createBag(70);let modelCurrent=pullBag(model),modelHold=null;
  let locks=0,holds=0,decisions=0,draws=1;
  while(locks<8&&decisions<30){
    const req=request(branch,5000),result=analyze(req);decisions++;
    let chosen=selectReachableSnapshotAction(branch,result,deps);
    if(decisions===1){
      const h=result.candidates.find(c=>c.action.kind==='hold');
      if(h)chosen={...validateSnapshotAction(branch,h.action,deps),candidate_index:result.candidates.indexOf(h)};
    }
    if(chosen.action.kind==='hold'){
      assert.ok(branch.hold());holds++;
      if(modelHold===null){modelHold=modelCurrent;modelCurrent=pullBag(model);draws++;}
      else [modelCurrent,modelHold]=[modelHold,modelCurrent];
      assert.equal(request(branch).hold_locked,true);
    }else{
      const timed=validateSnapshotTimingAction(branch,chosen.action,{...deps,framesPerPiece:24});
      for(let frame=timed.startFrame;frame<=timed.lockFrame;frame++)branch.step(tools.inputsForFrame(timed.inputs,frame));
      modelCurrent=pullBag(model);draws++;locks++;
    }
    assert.equal(branch.state.piece.type,modelCurrent);
    assert.equal(branch.state.hold.piece,modelHold);
    assert.deepEqual(branch.state.bag.queue.slice(0,5),model.queue.slice(0,5));
    assert.equal(request(branch).start.queue.length,6);
    assert.equal(recorded.serialize(),recordedBytes);
  }
  assert.equal(locks,8);assert.ok(draws>6);
}
checks.push('progressive reveal continues beyond six pieces without altering recorded checkpoint');

// Determinism across unrelated prior analyses.
{
  const e=make(80),r=request(e,5000);
  const a=analyze(r);analyze(request(make(81),5000));const b=analyze(r);
  assert.deepEqual(a,b);
}
checks.push('same allowed request is independent of prior analyses/replay history');

// Default 200k hard cap, actual nodes and completion reason.
const browserEngine=make(90);
const browserRequest=buildSnapshotRequest(captureSnapshotFromEngine(browserEngine,tools));
const full=analyze(browserRequest);
assert.equal(browserRequest.node_budget,200000);assert.ok(full.nodes<=200000);
assert.ok(['node_budget','visible_search_idle'].includes(full.completion));
fs.writeFileSync('snapshot-browser-request.json',JSON.stringify(browserRequest,null,2)+'\n');
fs.writeFileSync('snapshot-example-result.json',JSON.stringify(full,null,2)+'\n');
const report={status:'passed',schema:'kiwi-snapshot-acceptance/3',checks,
  default_budget:{nodes:full.nodes,budget:full.node_budget,completion:full.completion},
  capabilities:caps,rule_contract:snapshotRuleContract(),
  scope:'Phase 4A contract/fixture validation only; no user-visible Phase 4B and no strategy experiment'};
console.log(JSON.stringify(report,null,2));
