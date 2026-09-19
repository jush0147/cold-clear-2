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

const baseCells = {
  I:[[-1,0],[0,0],[1,0],[2,0]],
  O:[[0,0],[1,0],[0,1],[1,1]],
  T:[[-1,0],[0,0],[1,0],[0,1]],
  L:[[-1,0],[0,0],[1,0],[1,1]],
  J:[[-1,0],[0,0],[1,0],[-1,1]],
  S:[[-1,0],[0,0],[0,1],[1,1]],
  Z:[[-1,1],[0,1],[0,0],[1,0]],
};
function rotateCell(c, orientation) {
  const x=c[0], y=c[1];
  if (orientation === 'north') return [x,y];
  if (orientation === 'east') return [y,-x];
  if (orientation === 'south') return [-x,-y];
  if (orientation === 'west') return [-y,x];
  throw new Error('unknown CC2 rotation '+orientation);
}
function targetFor(placement) {
  const l = placement.location;
  const cells = baseCells[l.type].map(c => rotateCell(c,l.orientation))
    .map(c => [l.x+c[0], 39-(l.y+c[1])]);
  return { cells, spin:placement.spin, type:l.type.toLowerCase() };
}
const cellsKey = cells => cells.map(c => c[0]+','+Math.ceil(c[1])).sort().join(';');

function copyPiece(p) { return structuredClone(p); }
function dropped(board, piece) {
  const p = copyPiece(piece);
  while (B.legal(board,{...p,y:p.y+1})) p.y += 1;
  return p;
}
function pathStateKey(p) {
  // Do not key on totalRotations. Rotation loops would otherwise create an
  // artificial unbounded state dimension before the useful tuck is reached.
  return [p.x, Number(p.y).toFixed(6), p.r, p.kick, p.rotated?1:0, p.spin].join(',');
}
function applyPathMove(board, piece, action, ruleset) {
  let p = copyPiece(piece);
  if (action === 'moveLeft' || action === 'moveRight') {
    const x = p.x + (action === 'moveLeft' ? -1 : 1);
    if (!B.legal(board,{...p,x})) return null;
    p.x=x; p.rotated=false; p.spin='none'; p.wall=false; p.resets=(p.resets||0)+1; p.locking=0;
    return p;
  }
  if (action === 'down') {
    const q={...p,y:p.y+1};
    if (!B.legal(board,q)) return null;
    p=q; p.rotated=false; p.spin='none';
    return p;
  }
  const dir = action === 'rotateCW' ? 1 : action === 'rotateCCW' ? 3 : 2;
  if (dir===2 && !ruleset.allow180) return null;
  const q=R.rotate(board,p,dir,ruleset.lockresets);
  if (!q) return null;
  p={...p,...q,rotated:true,totalRotations:(p.totalRotations||0)+1,
    rotationResets:Math.min(63,(p.rotationResets||0)+1),resets:(p.resets||0)+1,locking:0};
  p.spin=R.classifySpin(board,p,ruleset.spinbonuses);
  return p;
}
function findPath(engine, placement) {
  const target=targetFor(placement);
  const root=Engine.restore(engine.serialize());
  const useHold = upperPiece(root.state.piece.type) !== placement.location.type;
  const prefix=[];
  if (useHold) {
    if (!root.hold()) throw new Error('Bot requested hold but authority could not hold');
    prefix.push('hold');
  }
  if (root.state.piece.type !== target.type) {
    throw new Error('piece mismatch after hold: authority='+root.state.piece.type+' target='+target.type);
  }
  const targetKey=cellsKey(target.cells), board=root.state.board;
  const q=[{piece:copyPiece(root.state.piece),moves:[]}];
  const seen=new Set();
  const actions=['moveLeft','moveRight','rotateCW','rotateCCW','rotate180','down'];
  let head=0;
  while(head<q.length && head<100000) {
    const node=q[head++], p=node.piece, key=pathStateKey(p);
    if(seen.has(key)) continue;
    seen.add(key);
    const drop=dropped(board,p);
    const spin=p.rotated ? R.classifySpin(board,p,root.state.rules.spinbonuses) : 'none';
    if(cellsKey(B.cells(drop))===targetKey && spin===target.spin) {
      return { useHold, moves:[...prefix,...node.moves,'hardDrop'], target };
    }
    if(node.moves.length>=32) continue;
    for(const action of actions) {
      const next=applyPathMove(board,p,action,root.state.rules);
      if(next) q.push({piece:next,moves:[...node.moves,action]});
    }
  }
  throw new Error('no Tetrp input path for '+JSON.stringify({placement,target,current:root.state.piece,boardTop:board.rows.findIndex(r=>r.some(Boolean))}));
}

function schedulePath(startFrame, lockFrame, moves) {
  const inputs=[];
  let frame=startFrame, sub=0.05;
  const tap = key => {
    if (sub>0.75) { frame++; sub=0.05; }
    inputs.push({frame,type:'keydown',key,subframe:sub});
    inputs.push({frame,type:'keyup',key,subframe:sub+0.05});
    sub+=0.15;
  };
  for (const move of moves) {
    if (move === 'hardDrop') continue;
    if (move === 'down') {
      if (sub>0.05) { frame++; sub=0.05; }
      inputs.push({frame,type:'keydown',key:'softDrop',subframe:0});
      inputs.push({frame:frame+1,type:'keyup',key:'softDrop',subframe:0});
      frame++; sub=0.05;
    } else {
      tap(move);
    }
  }
  if(frame>=lockFrame) throw new Error('path needs too much synthetic time: start='+startFrame+' pathFrame='+frame+' lock='+lockFrame);
  inputs.push({frame:lockFrame,type:'keydown',key:'hardDrop',subframe:0.5});
  inputs.push({frame:lockFrame,type:'keyup',key:'hardDrop',subframe:0.6});
  return inputs;
}
function addReplayInput(slot,input) {
  replayEvents[slot].push({
    frame:input.frame,type:input.type,
    data:{key:input.key,subframe:input.subframe},
    _order:eventOrder++,
  });
}
function engineInputsForFrame(inputs, frame) {
  return inputs.filter(e=>e.frame===frame).map(e=>({
    frame:e.frame,type:e.type,key:e.key,subframe:e.subframe
  }));
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
      engines[slot].step(engineInputsForFrame(plans[slot].inputs,frame));
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
