# Offline review worker

Serve `web/` beside the generated `pkg/` directory over HTTP(S).

```js
const worker = new Worker('./web/review.worker.js', {type:'module'});
const id = 'replay-position-123';
worker.postMessage({type:'review',id,request:{
  start:{board,queue:[current,...nextFive],hold,combo:consecutiveClears,
    back_to_back:hasB2B,b2b_count:visibleB2B,randomizer:{type:'unknown'}},
  incoming:[{lines:8,active:true}],pieces_placed:30,garbage_sent:20,
  actual:{placement:playerPlacement,use_hold:playerUsedHold},
  depth:4,beam_width:8
}});
worker.onmessage=({data})=>{
  if(data.id!==id)return;
  // "progress" rankings are incomplete and must not grade the player.
  if(data.type==='complete'){
    console.log(data.report.comparison);
    for(const action_id of new Set([
      data.report.comparison?.actual_action_id,
      data.report.candidates[0]?.action_id
    ]))if(action_id!==undefined)worker.postMessage({type:'details',id,action_id});
  }
  if(data.type==='details')console.log(data.details.lines);
};
// On seek, submit a new id. Old reports can be ignored by id.
// worker.postMessage({type:'cancel',id});
```

The session yields between root actions. Cancel latency is bounded by the current
root's work, not an instantaneous interruption inside a WASM call. A caller that
needs hard cancellation can terminate and recreate its Worker. Native/Node WASM
regressions do not certify browser layout or responsiveness on every device.

All rows are bottom-up occupancy strings in details. Incoming holes and resulting
boards are hypothetical scenario outputs, not future recorded facts. Root actions
are compared at the same finite lock depth and widths. Node counts and work time
can differ. The policy can adapt only after observable scenario states diverge.
The future preview shrinks in a displayed plan: unknown newly revealed pieces are
not read from the replay. `assessment` is a diagnostic, never an automatic grade.
