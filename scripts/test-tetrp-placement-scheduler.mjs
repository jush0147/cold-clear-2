import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { createPlacementTools } from './lib/tetrp-placement-path.mjs';

const tetrpRoot=path.resolve(process.argv[2]||'tetrp-reference');
const {Engine}=await import(pathToFileURL(path.join(tetrpRoot,'src/engine.js')).href);
const B=await import(pathToFileURL(path.join(tetrpRoot,'src/board.js')).href);
const R=await import(pathToFileURL(path.join(tetrpRoot,'src/rotation.js')).href);

const handling={
  arr:0,das:1,dcd:0,sdf:20,
  safelock:false,cancel:false,may20g:true,
  irs:'off',ihs:'off',
};
const makeEngine=seed=>new Engine({mode:'tl',seed,rules:{g:0,gincrease:0},handling});
const {schedulePath,inputsForFrame}=createPlacementTools({Engine,boardModule:B,rotationModule:R});

function runThrough(engine,inputs,lastFrame) {
  for(let frame=0;frame<=lastFrame;frame++) engine.step(inputsForFrame(inputs,frame));
}

// Capacity check: even a deliberately absurd 32-step descent must fit inside
// a 20-frame (3 PPS) transport window. Only soft-drop segments consume
// synthetic authority time, so the path is right-aligned near the lock frame.
const packed=schedulePath(0,19,[...Array(32).fill('down'),'hardDrop']);
const preLock=packed.filter(e=>e.key!=='hardDrop');
assert.ok(preLock.every(e=>e.frame<=19));
assert.ok(Math.min(...preLock.map(e=>e.frame))>=16,'32 soft-drop taps should need at most four source frames');
assert.deepEqual(
  packed.filter(e=>e.key==='hardDrop').map(e=>[e.frame,e.subframe]),
  [[19,0.5],[19,0.5]],
  'hard drop must stay at the scheduled lock instant'
);

// Semantic check against the pinned authority: eight soft-drop taps must move
// exactly eight rows while gravity remains zero.
const semantic=schedulePath(0,19,[...Array(8).fill('down'),'hardDrop']);
const movementOnly=semantic.filter(e=>e.key!=='hardDrop');
const movementEngine=makeEngine(777);
const y0=movementEngine.state.piece.y;
runThrough(movementEngine,movementOnly,19);
assert.equal(movementEngine.state.stats.pieces,0,'movement-only transport must not lock a piece');
assert.equal(movementEngine.state.piece.y-y0,8,'soft-drop taps must preserve one-row-per-down semantics');

const lockEngine=makeEngine(777);
for(let frame=0;frame<19;frame++) lockEngine.step(inputsForFrame(semantic,frame));
assert.equal(lockEngine.state.stats.pieces,0,'hard drop must not happen before the scheduled lock frame');
lockEngine.step(inputsForFrame(semantic,19));
assert.equal(lockEngine.state.stats.pieces,1,'piece must lock exactly on the scheduled lock frame');

// Regression for the H9 +1 / seeds 65206,65207 failure. A grounded finesse can
// legitimately contain more than 15 synthetic taps in this placement-only
// transport. Spacing every tap across source time caused Tetrp's reset-exhaustion
// check to auto-lock the first piece, then the scheduled hard drop locked a
// second piece. Equal-subframe taps must preserve the path and produce one lock.
const resetEngine=makeEngine(779);
while(B.legal(resetEngine.state.board,{...resetEngine.state.piece,y:resetEngine.state.piece.y+1})) {
  resetEngine.state.piece.y+=1;
}
resetEngine.state.piece.hy=Math.ceil(resetEngine.state.piece.y);
const resetMoves=[];
for(let i=0;i<8;i++) resetMoves.push('moveLeft','moveRight');
const resetPacked=schedulePath(0,19,[...resetMoves,'hardDrop']);
for(let frame=0;frame<19;frame++) resetEngine.step(inputsForFrame(resetPacked,frame));
assert.equal(resetEngine.state.stats.pieces,0,'reset-heavy path must not auto-lock before the scheduled frame');
resetEngine.step(inputsForFrame(resetPacked,19));
assert.equal(resetEngine.state.stats.pieces,1,'reset-heavy transport must lock exactly one piece');

// High-speed robustness uses 17 frames/piece = 60/17 ~= 3.529 PPS.
// Verify the same transport invariants in that narrower window.
const fastPacked=schedulePath(0,16,[...Array(32).fill('down'),'hardDrop']);
assert.ok(fastPacked.filter(e=>e.key!=='hardDrop').every(e=>e.frame<=16),'3.53 PPS path must fit the lock frame');

const fastSemantic=schedulePath(0,16,[...Array(8).fill('down'),'hardDrop']);
const fastMovement=makeEngine(778);
const fastY0=fastMovement.state.piece.y;
runThrough(fastMovement,fastSemantic.filter(e=>e.key!=='hardDrop'),16);
assert.equal(fastMovement.state.stats.pieces,0,'3.53 PPS movement-only path must not lock');
assert.equal(fastMovement.state.piece.y-fastY0,8,'3.53 PPS transport must preserve soft-drop semantics');

const fastLock=makeEngine(778);
for(let frame=0;frame<16;frame++) fastLock.step(inputsForFrame(fastSemantic,frame));
assert.equal(fastLock.state.stats.pieces,0,'3.53 PPS hard drop must not happen before frame 16');
fastLock.step(inputsForFrame(fastSemantic,16));
assert.equal(fastLock.state.stats.pieces,1,'3.53 PPS piece must lock on frame 16');

console.log(JSON.stringify({
  ok:true,
  checks:[
    '32-step soft-drop path fits 20-frame transport window',
    'right-aligned soft-drop taps preserve one-row-per-down semantics',
    'hard drop stays at the scheduled lock instant',
    'reset-heavy grounded finesse cannot auto-lock an extra piece',
    '17-frame high-speed transport preserves path and hard-drop semantics'
  ]
},null,2));
