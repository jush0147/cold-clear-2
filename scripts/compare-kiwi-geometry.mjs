// Compare against the unchanged v3.1 helper extracted with git show:
// git show 60e7539:scripts/lib/tetrp-placement-path.mjs > /tmp/kiwi-v31.mjs
// node scripts/compare-kiwi-geometry.mjs <tetrp-checkout> /tmp/kiwi-v31.mjs
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createPlacementTools} from './lib/tetrp-placement-path.mjs';
const root=resolve(process.argv[2]||'tetrp-reference');
const load=p=>import(pathToFileURL(resolve(root,'src',p)).href);
const {Engine}=await load('engine.js'),B=await load('board.js'),R=await load('rotation.js');
const {createPlacementTools:reference}=await import(pathToFileURL(resolve(process.argv[3])).href);
const deps={Engine,boardModule:B,rotationModule:R},before=reference(deps),after=createPlacementTools(deps);
const fixtures=[];
for(const type of ['i','o','t','l','j','s','z']){
  const e=new Engine({mode:'tl',seed:42});e.state.piece.type=type;
  fixtures.push([type,e]);
}
for(const count of [29,30,31]){
  const e=new Engine({mode:'tl',seed:44});e.state.piece.type='t';
  e.rotate(1);e.descend(5);while(e.move(-1)){}
  e.state.piece.totalRotations=count;
  fixtures.push(['rotated_wall_fractional_counter_'+count,e]);
}
{
  const e=new Engine({mode:'tl',seed:44});
  Object.assign(e.state.piece,{type:'t',x:4,y:20,hy:20,r:0,rotated:true,totalRotations:1});
  const cells=new Set(B.cells(e.state.piece).map(([x,y])=>x+','+Math.ceil(y)));
  for(let y=18;y<=22;y++)for(let x=2;x<=6;x++)if(!cells.has(x+','+y))e.state.board.rows[y][x]='i';
  e.state.piece.spin=R.classifySpin(e.state.board,e.state.piece,e.state.rules.spinbonuses);
  fixtures.push(['immobile_spin',e]);
}
const results=[];
for(const [name,e] of fixtures){
  const checkpoint=e.serialize();
  const start=performance.now(),a=before.enumerateRootPlacements(e),mid=performance.now();
  const b=after.enumerateRootPlacements(e),end=performance.now();
  assert.deepEqual(b,a,name);assert.equal(e.serialize(),checkpoint,name+' mutated input');
  results.push({name,beforeMs:mid-start,afterMs:end-mid,states:b.states_explored,placements:b.placements.length});
}
console.log(JSON.stringify({status:'passed',reference:'60e75395be109e58d255a85ecd1cf0c981e696b3',
  equality:'all placements, ordering, state counts, metadata; no input mutation',results},null,2));
