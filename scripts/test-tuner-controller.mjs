import assert from 'node:assert/strict';
import { askGeneration, initialState, profileForVector, tellGeneration } from './tuner/controller.mjs';

const config={
  schema:'tetrp-tuner-config/1',
  evidence_role:'non_evidentiary_controller_dry_run',
  algorithm:'diagonal_es_v1',
  random_seed:20260919,
  population:4,
  elite_fraction:0.5,
  pairs_per_candidate:2,
  pair_seed_base:71000,
  pair_seed_stride:20,
  parameters:{
    h9_cavity_excavation:{min:-0.625,max:-0.125,initial_mean:-0.375,initial_sigma:0.125,min_sigma:0.02,max_sigma:0.25}
  }
};
const a=askGeneration(config);
const b=askGeneration(config);
assert.deepEqual(a,b,'ask must be deterministic for the same state');
assert.equal(a.candidates.length,4);
assert.ok(a.candidates.every(c=>c.profile.startsWith('tuner:')));
assert.equal(
  profileForVector({h9_cavity_excavation:-0.375},['h9_cavity_excavation']),
  'tuner:{"h9_cavity_excavation":-0.375}'
);

const rows=[];
for(let ci=0;ci<a.candidates.length;ci++) {
  const c=a.candidates[ci];
  for(let pair=0;pair<c.pair_count;pair++) {
    const seeds=[c.pair_seed_base+pair*2,c.pair_seed_base+pair*2+1];
    for(let game=0;game<2;game++) {
      rows.push({
        type:'tetrp_synchronous_match',
        winner_profile:ci===0?c.profile:(game===0?c.profile:'corrected_legacy_h12'),
        screen_pair:{candidate_profile:c.profile,pair_seeds:seeds,candidate_slot:game},
      });
    }
  }
}
const told=tellGeneration(config,a,rows);
assert.equal(told.complete,true);
assert.equal(told.ranking[0],a.candidates[0].candidate_id);
assert.equal(told.next_state.generation,1);
const next=askGeneration(config,told.next_state);
assert.equal(next.generation,1);
assert.equal(next.candidates[0].pair_seed_base,71020);
assert.notDeepEqual(next.state.mean,initialState(config).mean);

console.log(JSON.stringify({ok:true,generation0:a.candidates.length,next_generation:next.generation}));
