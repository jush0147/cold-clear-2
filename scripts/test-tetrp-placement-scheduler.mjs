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
const engine=new Engine({mode:'tl',seed:777,rules:{g:0,gincrease:0},handling});
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
const y0=engine.state.piece.y;
for(let frame=0;frame<19;frame++) engine.step(inputsForFrame(semantic,frame));
assert.equal(engine.state.stats.pieces,0,'hard drop must not happen before the scheduled lock frame');
assert.equal(engine.state.piece.y-y0,8,'packed soft-drop taps must preserve one-row-per-down semantics');

engine.step(inputsForFrame(semantic,19));
assert.equal(engine.state.stats.pieces,1,'piece must lock exactly on the scheduled lock frame');

console.log(JSON.stringify({
  ok:true,
  checks:[
    '32-step path fits 20-frame transport window',
    'subframe soft-drop tap moves exactly one row at g=0/SDF20',
    'hard drop stays on scheduled lock frame'
  ]
},null,2));
