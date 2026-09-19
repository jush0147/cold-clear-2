import {
  SevenBagObserver,
  buildAnalysisRequest,
  captureVisibleState,
  drawsAdvancedByPlacement,
  visibleBagSix,
  visibleFingerprint,
} from './tetrp-authority-adapter.mjs';
import { createPlacementTools } from './tetrp-placement-path.mjs';

export function runSynchronousMatch({
  Engine,
  boardModule,
  rotationModule,
  analyzeProfileJson,
  seed,
  nodeBudget,
  framesPerPiece,
  profiles,
  tetrpRef = null,
  trace = false,
  safetyLockSteps = 5000,
}) {
  if (!Number.isSafeInteger(seed)) throw new Error('seed must be a safe integer');
  if (!Number.isInteger(nodeBudget) || nodeBudget < 1000) throw new Error('invalid node budget');
  if (!Number.isInteger(framesPerPiece) || framesPerPiece < 1) throw new Error('invalid frames per piece');
  if (!Array.isArray(profiles) || profiles.length !== 2) throw new Error('exactly two profiles are required');

  const handling = {
    arr:0,das:1,dcd:0,sdf:20,
    safelock:false,cancel:false,may20g:true,
    irs:'off',ihs:'off',
  };
  // Placement speed/gravity is deliberately neutralized. Frame time remains
  // authoritative for garbage travel and late-round attack scaling.
  const rules = {g:0,gincrease:0};
  const makeEngine=()=>new Engine({mode:'tl',seed,rules,handling});
  const engines=[makeEngine(),makeEngine()];
  const observers=engines.map(e=>SevenBagObserver.fromGameStart(visibleBagSix(e.state)));
  const {findPath,schedulePath,inputsForFrame}=createPlacementTools({
    Engine,boardModule,rotationModule
  });

  let queuedTransfers=[];
  let lockStep=0;
  const traces=[];
  const searchNodes=[0,0];

  function deliverTransfers() {
    const transfers=queuedTransfers;
    queuedTransfers=[];
    for(const t of transfers) {
      const receiver=engines[t.to];
      const cid=receiver.receive({
        from:'P2',iid:t.iid,ackiid:t.ackiid,amt:t.amt
      });
      receiver.confirm(cid);
    }
  }

  function collectOutbox() {
    const transfers=[];
    for(let from=0;from<2;from++) {
      const out=engines[from].state.attack.outbox.splice(0);
      for(const x of out) {
        transfers.push({from,to:1-from,iid:x.iid,ackiid:x.ackiid,amt:x.amt});
      }
    }
    return transfers;
  }

  while(true) {
    if(lockStep >= safetyLockSteps) {
      throw new Error('authority match exceeded safety lock-step cap; do not score this seed');
    }
    const startFrame=lockStep*framesPerPiece;
    const lockFrame=startFrame+framesPerPiece-1;
    for(const e of engines) {
      if(e.state.frame!==startFrame) {
        throw new Error('authority frame drift '+e.state.frame+' != '+startFrame);
      }
    }

    deliverTransfers();
    if(!engines[0].state.playing || !engines[1].state.playing) break;

    // Hard synchronous barrier. Both snapshots are captured before either
    // search runs, and the engines are checked again before commit.
    const visible=engines.map(captureVisibleState);
    const authorityBefore=engines.map(e=>e.serialize());
    const plans=[];

    for(let slot=0;slot<2;slot++) {
      const request=buildAnalysisRequest(visible[slot],observers[slot],{
        nodeBudget,framesPerPiece
      });
      const report=JSON.parse(analyzeProfileJson(JSON.stringify(request),profiles[slot]));
      if(!report.per_packet_ready_timing) {
        throw new Error('per-packet garbage timing was lost before search');
      }
      if(!report.authority_attack_clock) {
        throw new Error('Tetrp attack scaling clock was lost before search');
      }
      if(report.nodes>nodeBudget) throw new Error('hard node budget exceeded');
      if(!report.candidates.length) throw new Error('search produced no candidate');
      searchNodes[slot]+=report.nodes;
      const placement=report.candidates[0].placement;
      const path=findPath(engines[slot],placement);
      const inputs=schedulePath(startFrame,lockFrame,path.moves);
      plans.push({
        placement,path,inputs,report,
        drawsAdvanced:drawsAdvancedByPlacement(visible[slot],placement),
      });
    }

    for(let slot=0;slot<2;slot++) {
      if(engines[slot].serialize()!==authorityBefore[slot]) {
        throw new Error('authority mutated during pre-commit search for slot '+slot);
      }
    }

    for(let frame=startFrame;frame<=lockFrame;frame++) {
      // Both choices already exist. Slot iteration here cannot leak the first
      // placement's attack into the second decision; transfers are collected
      // only after both engines finish the lock frame.
      for(let slot=0;slot<2;slot++) {
        engines[slot].step(inputsForFrame(plans[slot].inputs,frame));
      }
    }

    for(let slot=0;slot<2;slot++) {
      if(engines[slot].state.playing) {
        observers[slot].advance(
          visibleBagSix(engines[slot].state),
          plans[slot].drawsAdvanced
        );
      }
    }

    queuedTransfers=collectOutbox();

    if(trace) {
      traces.push({
        lock_step:lockStep+1,
        frame:lockFrame,
        slots:[0,1].map(slot=>({
          profile:profiles[slot],
          before:visibleFingerprint(visible[slot]),
          placement:plans[slot].placement,
          path:plans[slot].path.moves,
          bag:observers[slot].snapshot(),
        }))
      });
    }

    lockStep++;
    if(!engines[0].state.playing || !engines[1].state.playing) break;
  }

  const alive=engines.map(e=>e.state.playing);
  const winnerSlot=alive[0]&&!alive[1]?0:alive[1]&&!alive[0]?1:null;
  const summarize=(slot)=>{
    const s=engines[slot].state;
    return {
      profile:profiles[slot],
      playing:s.playing,
      reason:s.reason,
      pieces:s.stats.pieces,
      holds:s.stats.holds,
      lines:s.stats.lines,
      generated:s.attack.totals.generated,
      cancelled:s.attack.totals.cancelled,
      sent:s.attack.totals.sent,
      tanked:s.attack.totals.tanked,
      received:s.attack.totals.received,
      raw_app:s.stats.pieces?s.attack.totals.generated/s.stats.pieces:0,
      sent_app:s.stats.pieces?s.attack.totals.sent/s.stats.pieces:0,
      search_nodes:searchNodes[slot],
      final_multiplier:s.attack.multiplier,
    };
  };

  const ruleState=engines[0].state.rules;
  return {
    result:{
      type:'tetrp_synchronous_match',
      seed,
      tetrp_ref:tetrpRef,
      node_budget_per_decision:nodeBudget,
      frames_per_piece:framesPerPiece,
      pps:60/framesPerPiece,
      zero_gravity:true,
      synchronous_barrier:true,
      per_packet_ready_timing:true,
      profiles:[...profiles],
      winner_slot:winnerSlot,
      winner_profile:winnerSlot===null?null:profiles[winnerSlot],
      lock_steps:lockStep,
      rules:{
        garbagespeed_frames:ruleState.garbagespeed_frames,
        garbagecap:ruleState.garbagecap,
        garbagemargin_frames:ruleState.garbagemargin_frames,
        garbageincrease_per_second:ruleState.garbageincrease_per_second,
        openerphase_pieces:ruleState.openerphase_pieces,
        garbageblocking:ruleState.garbageblocking,
        b2bcharging:ruleState.b2bcharging,
        passthrough:ruleState.passthrough,
      },
      slots:[summarize(0),summarize(1)],
    },
    trace:traces,
  };
}
