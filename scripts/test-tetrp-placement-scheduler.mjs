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

// Capacity check: even a deliberately absurd 32-step descent must fit inside
// a 20-frame (3 PPS) transport window. CC2 movegen caps path length at 32.
const packed=schedulePath(0,19,[...Array(32).fill('down'),'hardDrop']);
const preLock=packed.filter(e=>e.key!=='hardDrop');
assert.ok(preLock.every(e=>e.frame<19));
assert.ok(Math.max(...preLock.map(e=>e.frame))<=6,'32 taps should pack into at most 7 source frames');

// Semantic check against the pinned authority: eight packed soft-drop taps
// must move exactly eight rows while gravity remains zero.
const semantic=schedulePath(0,19,[...Array(8).fill('down'),'hardDrop']);
const engine=makeEngine(777);
const y0=engine.state.piece.y;
for(let frame=0;frame<19;frame++) engine.step(inputsForFrame(semantic,frame));
assert.equal(engine.state.stats.pieces,0,'hard drop must not happen before the scheduled lock frame');
assert.equal(engine.state.piece.y-y0,8,'packed soft-drop taps must preserve one-row-per-down semantics');
engine.step(inputsForFrame(semantic,19));
assert.equal(engine.state.stats.pieces,1,'piece must lock exactly on the scheduled lock frame');

// Regression for H9 +1 / seeds 65206,65207. The ordinary tap spacing can
// cross Tetrp's 15-reset auto-lock on a grounded, reset-heavy placement. When
// given the live authority state schedulePath must detect that and use compact
// equal-subframe transport instead, so the planned hard drop locks exactly one piece.
const resetEngine=makeEngine(779);
while(B.legal(resetEngine.state.board,{...resetEngine.state.piece,y:resetEngine.state.piece.y+1})) {
  resetEngine.state.piece.y+=1;
}
resetEngine.state.piece.hy=Math.ceil(resetEngine.state.piece.y);
const resetMoves=[];
for(let i=0;i<8;i++) resetMoves.push('moveLeft','moveRight');
const resetPacked=schedulePath(0,19,[...resetMoves,'hardDrop'],resetEngine);
const resetPreLock=resetPacked.filter(e=>e.key!=='hardDrop');
assert.ok(resetPreLock.every(e=>e.frame===19 && e.subframe===0.5),
  'reset-heavy fallback taps must share the scheduled hard-drop subframe');
for(let frame=0;frame<19;frame++) resetEngine.step(inputsForFrame(resetPacked,frame));
assert.equal(resetEngine.state.stats.pieces,0,'reset-heavy path must not auto-lock before the scheduled frame');
resetEngine.step(inputsForFrame(resetPacked,19));
assert.equal(resetEngine.state.stats.pieces,1,'reset-heavy transport must lock exactly one piece');

// High-speed robustness uses 17 frames/piece = 60/17 ~= 3.529 PPS.
// Verify the same ordinary transport invariants in that narrower window.
const fastEngine=makeEngine(778);
const fastPacked=schedulePath(0,16,[...Array(32).fill('down'),'hardDrop']);
const fastPreLock=fastPacked.filter(e=>e.key!=='hardDrop');
assert.ok(fastPreLock.every(e=>e.frame<16),'3.53 PPS path taps must precede lock frame');

const fastSemantic=schedulePath(0,16,[...Array(8).fill('down'),'hardDrop']);
const fastY0=fastEngine.state.piece.y;
for(let frame=0;frame<16;frame++) fastEngine.step(inputsForFrame(fastSemantic,frame));
assert.equal(fastEngine.state.stats.pieces,0,'3.53 PPS hard drop must not happen before the scheduled lock frame');
assert.equal(fastEngine.state.piece.y-fastY0,8,'3.53 PPS packed soft-drop taps must preserve movement semantics');
fastEngine.step(inputsForFrame(fastSemantic,16));
assert.equal(fastEngine.state.stats.pieces,1,'3.53 PPS piece must lock on frame 16');

console.log(JSON.stringify({
  ok:true,
  checks:[
    '32-step path fits 20-frame transport window',
    'subframe soft-drop tap moves exactly one row at g=0/SDF20',
    'hard drop stays on scheduled lock frame',
    'reset-heavy grounded finesse falls back to one-lock compact transport',
    '17-frame high-speed transport preserves path and hard-drop semantics'
  ]
},null,2));
