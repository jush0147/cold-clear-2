import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {captureSnapshot,buildSnapshotRequest,validateSnapshotAction,selectReachableSnapshotAction} from './lib/kiwi-snapshot-adapter.mjs';
import {createPlacementTools} from './lib/tetrp-placement-path.mjs';
const require=createRequire(import.meta.url);
const wasm=require('../pkg-node/cold_clear_2.js');
const root=path.resolve(process.argv[2]||'tetrp-reference');
const load=p=>import(pathToFileURL(path.join(root,'src',p)).href);
const {Engine}=await load('engine.js');
const B=await load('board.js'),R=await load('rotation.js');
const {createBag,pullBag}=await load('random.js');
const tools=createPlacementTools({Engine,boardModule:B,rotationModule:R});
const dependencies={Engine,placementTools:tools};
const make=seed=>new Engine({seed,mode:'tl',rules:{g:0,gincrease:0,b2bcharge_base:3},handling:{arr:0,das:1,dcd:0,sdf:20,safelock:false,cancel:false,may20g:true,irs:'off',ihs:'off'}});
const request=(e,nodes=5000)=>buildSnapshotRequest(captureSnapshot(e.state),{nodeBudget:nodes});
const analyze=r=>JSON.parse(wasm.analyze_snapshot_json(JSON.stringify(r)));
const checks=[];
const cap=JSON.parse(wasm.snapshot_capabilities_json());
assert.equal(cap.snapshot_api,'analyze_snapshot_json');
assert.equal(cap.history_derived_bag,false);assert.equal(cap.speculative_tail_expansion,false);
assert.equal(cap.same_piece_hold_search,false);assert.equal(cap.product_persistent_dag_reuse,false);

// Hidden state and prior history must not even be read by capture/builder.
const original=make(42),before=original.serialize();
const a=request(original),modified=JSON.parse(before);
modified.bag.queue=new Proxy(modified.bag.queue,{get(t,key){
  if(/^\d+$/.test(String(key))&&Number(key)>=5)throw new Error('Hidden NEXT read');
  return Reflect.get(t,key);
}});
Object.defineProperty(modified.bag,'rng',{get(){throw new Error('RNG read');}});
Object.defineProperty(modified,'observedDraws',{get(){throw new Error('History read');}});
Object.defineProperty(modified,'opponent',{get(){throw new Error('Opponent read');}});
const projected=captureSnapshot(modified);
Object.defineProperty(projected,'bag_state',{get(){throw new Error('Bag recovery');}});
assert.deepEqual(a,buildSnapshotRequest(projected,{nodeBudget:5000}));
const ar=analyze(a);
analyze(request(make(99)));
assert.deepEqual(ar,analyze(buildSnapshotRequest(projected,{nodeBudget:5000})));
assert.equal(original.serialize(),before);
for(const forbidden of ['randomizer','bag_state','observedDraws','rng','hidden_next','opponent']){
  assert.ok(!JSON.stringify(a).includes('"'+forbidden+'"'));
  assert.throws(()=>analyze({...a,[forbidden]:[]}));
}
checks.push('allowlisted request and deterministic result ignore history/hidden NEXT/RNG/opponent');

// Exact separate empty-Hold example; a hidden L is revealed only after Hold.
const ex=make(7);
ex.state.piece.type='t';ex.state.hold={piece:null,locked:false};
ex.state.bag.queue=['i','o','s','z','j','l','t','s','z','j','o','i'];
const pre=request(ex),exBefore=ex.serialize();
assert.deepEqual(pre.start.queue,['T','I','O','S','Z','J']);
const hold={kind:'hold',mode:'empty',requires_reanalysis:true};
assert.deepEqual(validateSnapshotAction(ex,hold,dependencies),{action:hold});
assert.equal(ex.serialize(),exBefore);
assert.throws(()=>validateSnapshotAction(ex,{...hold,placement:{fake:true}},dependencies));
assert.ok(ex.hold());
const post=request(ex);
assert.deepEqual(post.start.queue,['I','O','S','Z','J','L']);
assert.equal(post.start.hold,'T');assert.equal(post.hold_locked,true);
assert.equal(ex.state.stats.pieces,0);
const postResult=analyze(post);
assert.ok(postResult.candidates.every(c=>c.action.kind==='place'&&c.action.placement.location.type==='I'));
assert.ok(!ex.hold());
checks.push('empty Hold alone reveals one preview immediately; post-Hold request is locked and reanalyzed');

// Occupied Hold, including a same-type authority action, consumes no draw.
// Same-type Hold is accepted by Tetrp but not separately ranked by Kiwi search.
for(const same of [false,true]){
  const e=make(8);e.state.hold.piece=same?e.state.piece.type:'t';
  const q=JSON.stringify(e.state.bag),old=e.state.piece.type,held=e.state.hold.piece;
  const action={kind:'hold',mode:'occupied',requires_reanalysis:true};
  validateSnapshotAction(e,action,dependencies);assert.ok(e.hold());
  assert.equal(JSON.stringify(e.state.bag),q);assert.equal(e.state.piece.type,held);
  assert.equal(e.state.hold.piece,old);assert.equal(request(e).hold_locked,true);
}
checks.push('occupied Hold changes no sequence state; same-piece search exclusion is explicit');

// Independently consume the original sequence in an authority-only audit model.
// The model and original generator NEVER cross the request/Worker boundary.
const recorded=make(123),recordedBytes=recorded.serialize();
const other=make(321),otherBytes=other.serialize();
let branch=Engine.restore(recordedBytes);
const model=createBag(123);let modelCurrent=pullBag(model),modelHold=null;
let locks=0,holds=0,decisions=0,draws=1;
while(locks<10&&decisions<30){
  const req=request(branch,5000);const result=analyze(req);decisions++;
  assert.ok(result.nodes<=req.node_budget);
  let chosen=selectReachableSnapshotAction(branch,result,dependencies);
  if(decisions===1)chosen=validateSnapshotAction(branch,hold,dependencies);
  const oldPieces=branch.state.stats.pieces;
  if(chosen.action.kind==='hold'){
    assert.ok(branch.hold());holds++;
    if(modelHold===null){modelHold=modelCurrent;modelCurrent=pullBag(model);draws++;}
    else [modelCurrent,modelHold]=[modelHold,modelCurrent];
    assert.equal(branch.state.stats.pieces,oldPieces);
    assert.equal(request(branch).hold_locked,true);
  }else{
    const start=branch.state.frame,end=start+23;
    const inputs=tools.schedulePath(start,end,chosen.path.moves,branch);
    for(let frame=start;frame<=end;frame++)branch.step(tools.inputsForFrame(inputs,frame));
    assert.equal(branch.state.stats.pieces,oldPieces+1);
    assert.equal(branch.state.hold.locked,false);
    modelCurrent=pullBag(model);draws++;locks++;
  }
  assert.equal(branch.state.piece.type,modelCurrent);
  assert.equal(branch.state.hold.piece,modelHold);
  assert.deepEqual(branch.state.bag.queue.slice(0,5),model.queue.slice(0,5));
  assert.equal(request(branch).start.queue.length,6);
  assert.equal(recorded.serialize(),recordedBytes);assert.equal(other.serialize(),otherBytes);
}
assert.equal(locks,10);assert.ok(draws>6);assert.ok(holds>0);
branch=null;
assert.equal(recorded.serialize(),recordedBytes);
checks.push('ten lock/spawn cycles plus Holds refill NEXT 5 from original branch sequence, beyond initial window');
checks.push('isolated branch disposal leaves both recorded checkpoints unchanged');

// Pending and late clock use the SAME v2 path. Unknown activation is an error.
const pending=make(51);const cid=pending.receive({from:'P2',iid:1,ackiid:0,amt:4});
assert.throws(()=>request(pending),/activation/);
pending.confirm(cid);
const pr=request(pending),pResult=analyze(pr);
assert.equal(pResult.scenarios,10);assert.equal(pResult.authority_attack_clock,true);
assert.equal(pr.rules.b2bcharge_base,3);assert.ok(pResult.nodes<=5000);
const late=make(52);late.state.attack.multiplier=1.75;
const lr=request(late),lResult=analyze(lr);
assert.equal(lr.incoming.length,0);assert.equal(lr.garbage_multiplier,1.75);
assert.equal(lResult.search_path,'stateless_clocked_snapshot');assert.equal(lResult.authority_attack_clock,true);
const defaultRequest=buildSnapshotRequest(captureSnapshot(original.state));
assert.equal(defaultRequest.node_budget,200000);
const full=analyze(defaultRequest);assert.ok(full.nodes<=200000);assert.equal(full.node_budget,200000);
checks.push('pending, empty incoming with non-unit multiplier, public rules, default and small hard budgets');

// Save the actual allowlisted fixture for a real browser Worker test, not a fake bot.
fs.writeFileSync('snapshot-browser-request.json',JSON.stringify(defaultRequest));
const report={status:'passed',schema:'kiwi-snapshot-acceptance/2',checks,
  continuation:{locks,holds,decisions,draws},default_budget:{nodes:full.nodes,budget:full.node_budget},
  capabilities:cap,scope:'isolated protocol harness only; Tetrp Phase 4B not implemented'};
console.log(JSON.stringify(report,null,2));
