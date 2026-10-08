import test from 'node:test';
import assert from 'node:assert/strict';
import {chooseExecutableCandidate} from './lib/tetrp-executable-candidate.mjs';

const candidates=[{placement:{name:'top'}},{placement:{name:'second'}},{placement:{name:'third'}}];
const engine={state:'unchanged'};

test('uses highest-ranked candidate when executable',()=>{
  const selected=chooseExecutableCandidate(candidates,engine,10,33,{
    findPath:(actual,placement)=>{
      assert.equal(actual,engine);
      return {moves:['rotateCW','hardDrop'],target:placement.name};
    },
    schedulePath:(start,lock,moves,actual)=>{
      assert.deepEqual([start,lock,actual],[10,33,engine]);
      assert.deepEqual(moves,['rotateCW','hardDrop']);
      return [{type:'keydown',key:'hardDrop'}];
    },
  });
  assert.equal(selected.rank,0);
  assert.equal(selected.placement,candidates[0].placement);
  assert.equal(selected.inputs.length,1);
});

test('falls back when top geometry has no legal Tetrp path',()=>{
  const visited=[];
  const selected=chooseExecutableCandidate(candidates,engine,0,23,{
    findPath:(_,p)=>{
      visited.push(p.name);
      if(p.name==='top') throw new Error('no path');
      return {moves:['hardDrop']};
    },
    schedulePath:()=>[],
  });
  assert.deepEqual(visited,['top','second']);
  assert.equal(selected.rank,1);
  assert.equal(selected.placement.name,'second');
});

test('falls back when a geometrically legal path fails timed transport',()=>{
  const scheduled=[];
  const selected=chooseExecutableCandidate(candidates,engine,0,23,{
    findPath:(_,p)=>({moves:[p.name]}),
    schedulePath:(_,__,moves)=>{
      scheduled.push(moves[0]);
      if(moves[0]==='top') throw new Error('early lock');
      return [{move:moves[0]}];
    },
  });
  assert.deepEqual(scheduled,['top','second']);
  assert.equal(selected.rank,1);
});

test('rejects match rather than cheating when all ranked candidates fail',()=>{
  assert.throws(()=>chooseExecutableCandidate(candidates,engine,0,23,{
    findPath:()=>{throw new Error('unreachable')},
    schedulePath:()=>{throw new Error('must not run')},
  }),/no executable Tetrp candidate \(3 searched ranks\)/);
});

test('empty and malformed candidates cannot be executed',()=>{
  const tools={findPath:()=>{throw new Error('no path')},schedulePath:()=>[]};
  assert.throws(()=>chooseExecutableCandidate([],engine,0,23,tools),/no candidate/);
  assert.throws(()=>chooseExecutableCandidate([null,{}],engine,0,23,tools),/no executable Tetrp candidate/);
});
