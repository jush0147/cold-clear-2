import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import {
  SevenBagObserver,
  buildAnalysisRequest,
  captureVisibleState,
  drawsAdvancedByPlacement,
  upperPiece,
  visibleBagSix,
  visibleFingerprint,
} from './lib/tetrp-authority-adapter.mjs';
import { createPlacementTools } from './lib/tetrp-placement-path.mjs';

const require = createRequire(import.meta.url);
const { analyze_pending_profile_json } = require('../pkg-node/cold_clear_2.js');

const tetrpRoot = path.resolve(process.argv[2] || 'tetrp-reference');
const outPath = path.resolve(process.argv[3] || 'bot-exhibition-49000.ttrm');
const seed = Number(process.env.EXHIBITION_SEED || 49000);
const nodeBudget = Number(process.env.NODE_BUDGET || 200000);
const framesPerPiece = Number(process.env.FRAMES_PER_PIECE || 30);
const stopAfterPieces = process.env.STOP_AFTER_PIECES == null ? null : Number(process.env.STOP_AFTER_PIECES);
const assertMirror = process.env.ASSERT_MIRROR === '1';

const { Engine } = await import(pathToFileURL(path.join(tetrpRoot, 'src/engine.js')).href);
const B = await import(pathToFileURL(path.join(tetrpRoot, 'src/board.js')).href);
const R = await import(pathToFileURL(path.join(tetrpRoot, 'src/rotation.js')).href);

const handling = {
  arr: 0, das: 1, dcd: 0, sdf: 20,
  safelock: false, cancel: false, may20g: true,
  irs: 'off', ihs: 'off',
};
const rules = { g: 0, gincrease: 0 };
const profiles = [
  process.env.SLOT0_PROFILE || 'review_h9_h12',
  process.env.SLOT1_PROFILE || 'corrected_legacy_h12',
];
const names = [
  process.env.SLOT0_NAME || (profiles[0] === 'review_h9_h12' ? 'TUNED H9' : 'CORRECTED LEGACY'),
  process.env.SLOT1_NAME || (profiles[1] === 'review_h9_h12' ? 'TUNED H9' : 'CORRECTED LEGACY'),
];
const gameids = [0, 1];

function makeEngine() {
  return new Engine({ mode: 'tl', seed, rules, handling });
}
const engines = [makeEngine(), makeEngine()];
const observers = engines.map(e => SevenBagObserver.fromGameStart(visibleBagSix(e.state)));

const replayEvents = [
  [{ frame:0, type:'start', data:{}, _order:0 },
   { frame:0, type:'ige', data:{ id:1, frame:0, type:'target', data:{targets:[1]} }, _order:1 }],
  [{ frame:0, type:'start', data:{}, _order:0 },
   { frame:0, type:'ige', data:{ id:1, frame:0, type:'target', data:{targets:[0]} }, _order:1 }],
];
const nextIgeId = [2, 2];
const nextRemoteCid = [1, 1];
let eventOrder = 10;

const { findPath, schedulePath, inputsForFrame } = createPlacementTools({
  Engine, boardModule:B, rotationModule:R
});

function addReplayInput(slot,input) {
  replayEvents[slot].push({
    frame:input.frame,type:input.type,
    data:{key:input.key,subframe:input.subframe},
    _order:eventOrder++,
  });
}

let queuedTransfers=[];
function deliverTransfers(frame) {
  const transfers=queuedTransfers; queuedTransfers=[];
  for(const t of transfers) {
    const receiver=engines[t.to];
    const localCid=receiver.receive({from:'P2',iid:t.iid,ackiid:t.ackiid,amt:t.amt});
    // Even a fully-passthrough interaction must remain in the replay because
    // receive() applies ack bookkeeping to our outstanding outgoing packets.
    // A confirm mapped to null is intentionally a no-op in Reconstruction.
    receiver.confirm(localCid);

    const remoteCid=nextRemoteCid[t.to]++;
    const senderGameId=gameids[t.from];
    const interaction={
      frame,type:'ige',_order:eventOrder++,
      data:{id:nextIgeId[t.to]++,frame,type:'interaction',data:{
        type:'garbage',amt:t.amt,gameid:senderGameId,frame,cid:remoteCid,
        iid:t.iid,ackiid:t.ackiid,x:0,y:0,size:1
      }}
    };
    const confirm=structuredClone(interaction);
    confirm._order=eventOrder++;
    confirm.data.id=nextIgeId[t.to]++;
    confirm.data.type='interaction_confirm';
    replayEvents[t.to].push(interaction,confirm);
  }
}
function collectOutbox() {
  const transfers=[];
  for(let from=0;from<2;from++) {
    const out=engines[from].state.attack.outbox.splice(0);
    for(const x of out) transfers.push({from,to:1-from,iid:x.iid,ackiid:x.ackiid,amt:x.amt});
  }
  return transfers;
}

const diagnostics=[];
let pieceIndex=0;
let winner=null;
const maxPieces=stopAfterPieces ?? 800;
while(pieceIndex<maxPieces) {
  const startFrame=pieceIndex*framesPerPiece;
  const lockFrame=startFrame+framesPerPiece-1;
  for(const e of engines) {
    if(e.state.frame!==startFrame) throw new Error('authority frame drift '+e.state.frame+' != '+startFrame);
  }
  deliverTransfers(startFrame);

  if(!engines[0].state.playing || !engines[1].state.playing) break;

  // Synchronous decision barrier: capture both visible states before either
  // bot searches or either new placement is committed.
  const visible=engines.map(captureVisibleState);
  const authorityBefore=engines.map(e=>e.serialize());
  const fingerprints=visible.map(visibleFingerprint);

  const plans=[];
  for(let slot=0;slot<2;slot++) {
    const request=buildAnalysisRequest(visible[slot],observers[slot],{
      nodeBudget,framesPerPiece
    });
    const report=JSON.parse(analyze_pending_profile_json(JSON.stringify(request),profiles[slot]));
    if(!report.per_packet_ready_timing) {
      throw new Error('formal authority adapter must preserve per-packet garbage timing');
    }
    if(!report.candidates.length) throw new Error('no bot candidates for slot '+slot);
    const placement=report.candidates[0].placement;
    const pathResult=findPath(engines[slot],placement);
    const inputs=schedulePath(startFrame,lockFrame,pathResult.moves);
    inputs.forEach(e=>addReplayInput(slot,e));
    plans.push({
      placement,path:pathResult,inputs,report,
      drawsAdvanced:drawsAdvancedByPlacement(visible[slot],placement),
      preDecisionFingerprint:fingerprints[slot],
    });
  }

  // Search is external to the authority. If either engine changed while the
  // two decisions were being produced, the synchronous barrier was violated.
  for(let slot=0;slot<2;slot++) {
    if(engines[slot].serialize()!==authorityBefore[slot]) {
      throw new Error('authority mutated during pre-commit search for slot '+slot);
    }
  }

  for(let frame=startFrame;frame<=lockFrame;frame++) {
    for(let slot=0;slot<2;slot++) {
      engines[slot].step(inputsForFrame(plans[slot].inputs,frame));
    }
  }

  for(let slot=0;slot<2;slot++) {
    if(engines[slot].state.playing) {
      observers[slot].advance(visibleBagSix(engines[slot].state),plans[slot].drawsAdvanced);
    }
  }

  diagnostics.push({
    piece:pieceIndex+1,frame:lockFrame,
    slot0:{profile:profiles[0],pieces:engines[0].state.stats.pieces,
      app:engines[0].state.attack.totals.generated/Math.max(1,engines[0].state.stats.pieces),
      placement:plans[0].placement,path:plans[0].path.moves,
      pre_decision:plans[0].preDecisionFingerprint,
      bag:observers[0].snapshot()},
    slot1:{profile:profiles[1],pieces:engines[1].state.stats.pieces,
      app:engines[1].state.attack.totals.generated/Math.max(1,engines[1].state.stats.pieces),
      placement:plans[1].placement,path:plans[1].path.moves,
      pre_decision:plans[1].preDecisionFingerprint,
      bag:observers[1].snapshot()},
  });

  queuedTransfers=collectOutbox();
  if(assertMirror) {
    if(profiles[0]!==profiles[1]) throw new Error('ASSERT_MIRROR requires identical profiles');
    if(engines[0].serialize()!==engines[1].serialize()) {
      throw new Error('A/A authority states diverged after piece '+(pieceIndex+1));
    }
    if(JSON.stringify(observers[0].snapshot())!==JSON.stringify(observers[1].snapshot())) {
      throw new Error('A/A SevenBag observers diverged after piece '+(pieceIndex+1));
    }
  }
  if(!engines[0].state.playing || !engines[1].state.playing) {
    winner=engines[0].state.playing?0:engines[1].state.playing?1:null;
    pieceIndex++;
    break;
  }
  pieceIndex++;
}
if(pieceIndex>=maxPieces && stopAfterPieces == null) throw new Error('exhibition exceeded safety piece cap');
const endFrame=pieceIndex*framesPerPiece;
deliverTransfers(endFrame);

function endData(slot, reason) {
  const s=engines[slot].state, p=s.piece;
  let flags=128; // Reconstruction terminal semantics force the falling piece asleep.
  if(p.wall) flags|=64;
  if(p.forceLock) flags|=2048;
  if(p.softDropped) flags|=4096;
  if(p.spin==='full') flags|=8;
  else if(p.spin==='mini') flags|=24;
  return {
    gameoverreason:reason,
    stats:{lines:s.stats.lines,holds:s.stats.holds,piecesplaced:s.stats.pieces},
    game:{
      board:structuredClone(s.board.rows),
      bag:[...s.bag.queue],
      hold:structuredClone(s.hold),
      g:s.g,
      playing:false,
      falling:{
        type:p.type,x:p.x,y:p.y,r:p.r,hy:p.hy,kick:p.kick,keys:p.keys,
        safelock:p.safelock,locking:p.locking,lockresets:p.resets,
        rotresets:p.rotationResets,flags
      }
    }
  };
}
for(let slot=0;slot<2;slot++) {
  const reason=engines[slot].state.reason || (winner===slot?'winner':'synthetic-end');
  replayEvents[slot].push({frame:endFrame,type:'end',data:endData(slot,reason),_order:999999+slot});
}

function stream(slot) {
  const events=replayEvents[slot].sort((a,b)=>a.frame-b.frame||a._order-b._order)
    .map(e=>{const {_order,...rest}=e;return rest;});
  return {
    frames:endFrame,
    options:{
      version:19,seed,gameid:gameids[slot],handling,
      g:0,gincrease:0,hasgarbage:true,username:names[slot],
    },
    events,
  };
}
const file={
  version:1,id:null,gamemode:'league',verified:false,
  ts:new Date().toISOString(),
  users:[
    {id:'tuned',username:names[0],avatar_revision:0,banner_revision:0,flags:0,country:null},
    {id:'legacy',username:names[1],avatar_revision:0,banner_revision:0,flags:0,country:null},
  ],
  replay:{
    rounds:[[
      {id:'tuned',username:names[0],replay:stream(0)},
      {id:'legacy',username:names[1],replay:stream(1)},
    ]]
  },
  synthetic:{
    purpose:'visualize cold-clear-2 play without modifying Tetrp',
    seed,node_budget:nodeBudget,frames_per_piece:framesPerPiece,
    profiles,names,winner:winner===null?'simultaneous-topout':names[winner],
    authority:'jush0147/tetrp engine',
    stopped_after_pieces:stopAfterPieces,
    mirror_assertion:assertMirror,
    warning:'Synthetic exhibition/control only. Do not pool this match into strategy-strength evidence.',
    final:{
      tuned:{pieces:engines[0].state.stats.pieces,generated:engines[0].state.attack.totals.generated,sent:engines[0].state.attack.totals.sent},
      legacy:{pieces:engines[1].state.stats.pieces,generated:engines[1].state.attack.totals.generated,sent:engines[1].state.attack.totals.sent},
    }
  }
};
writeFileSync(outPath,JSON.stringify(file,null,2)+'\n');
writeFileSync(outPath+'.diagnostic.json',JSON.stringify({diagnostics,synthetic:file.synthetic},null,2)+'\n');
console.log(JSON.stringify(file.synthetic,null,2));
console.log(outPath);
