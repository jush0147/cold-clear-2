import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root=path.resolve(process.argv[2]||'confirmation-results');
const out=path.resolve(process.argv[3]||'fresh-confirmation-summary.json');

const TUNED='review_h9_h12';
const LEGACY='corrected_legacy_h12';
const primarySeeds=Array.from({length:16},(_,i)=>63000+i);
const robustness2Seeds=[63000,63001,63002,63003];
const robustness3Seeds=[63004,63005,63006,63007];
const expectedByFrames=new Map([
  [24,primarySeeds],
  [30,robustness2Seeds],
  [20,robustness3Seeds],
]);

function walk(dir){
  const out=[];
  for(const entry of readdirSync(dir,{withFileTypes:true})){
    const p=path.join(dir,entry.name);
    if(entry.isDirectory()) out.push(...walk(p));
    else if(entry.isFile() && entry.name.endsWith('.jsonl')) out.push(p);
  }
  return out;
}
function median(xs){
  const a=[...xs].sort((x,y)=>x-y);
  const m=Math.floor(a.length/2);
  return a.length%2?a[m]:(a[m-1]+a[m])/2;
}
function slot(r,profile){
  const s=r.slots.find(x=>x.profile===profile);
  if(!s) throw new Error('missing profile '+profile+' at seed '+r.seed+' pace '+r.frames_per_piece);
  return s;
}
function aggregate(rs,profile){
  const sums=rs.reduce((a,r)=>{
    const s=slot(r,profile);
    a.pieces+=s.pieces;
    a.generated+=s.generated;
    a.cancelled+=s.cancelled;
    a.sent+=s.sent;
    a.tanked+=s.tanked;
    a.received+=s.received;
    return a;
  },{pieces:0,generated:0,cancelled:0,sent:0,tanked:0,received:0});
  return {
    ...sums,
    raw_app:sums.pieces?sums.generated/sums.pieces:0,
    sent_app:sums.pieces?sums.sent/sums.pieces:0,
  };
}

const rows=[];
for(const file of walk(root)){
  for(const line of readFileSync(file,'utf8').split(/\r?\n/)){
    if(!line.trim()) continue;
    const row=JSON.parse(line);
    if(row.type==='tetrp_synchronous_match') rows.push(row);
  }
}
if(rows.length!==24) throw new Error('expected 24 scored matches, found '+rows.length);

const expectedKeys=new Set();
for(const [frames,seeds] of expectedByFrames){
  for(const seed of seeds) expectedKeys.add(seed+':'+frames);
}
const seen=new Set();
for(const r of rows){
  if(r.node_budget_per_decision!==200000) throw new Error('node budget drift at seed '+r.seed);
  if(r.synchronous_barrier!==true || r.per_packet_ready_timing!==true || r.zero_gravity!==true) {
    throw new Error('authority contract missing at seed '+r.seed);
  }
  if(r.winner_profile!==TUNED && r.winner_profile!==LEGACY) {
    throw new Error('non-unique/unknown winner at seed '+r.seed);
  }
  const k=r.seed+':'+r.frames_per_piece;
  if(!expectedKeys.has(k)) throw new Error('unexpected seed/pace '+k);
  if(seen.has(k)) throw new Error('duplicate seed/pace '+k);
  seen.add(k);
}
for(const k of expectedKeys) if(!seen.has(k)) throw new Error('missing seed/pace '+k);

const byPace={};
for(const [frames,seeds] of expectedByFrames){
  const rs=rows.filter(r=>r.frames_per_piece===frames).sort((a,b)=>a.seed-b.seed);
  const tunedWins=rs.filter(r=>r.winner_profile===TUNED).length;
  const legacyWins=rs.length-tunedWins;
  const locks=rs.map(r=>r.lock_steps);
  byPace[String(frames)]={
    frames_per_piece:frames,
    pps:60/frames,
    seeds,
    matches:rs.length,
    tuned_wins:tunedWins,
    legacy_wins:legacyWins,
    lock_steps:{
      avg:locks.reduce((a,b)=>a+b,0)/locks.length,
      median:median(locks),
      min:Math.min(...locks),
      max:Math.max(...locks),
      values:rs.map(r=>({seed:r.seed,lock_steps:r.lock_steps})),
    },
    tuned:aggregate(rs,TUNED),
    legacy:aggregate(rs,LEGACY),
    per_match:rs.map(r=>({
      seed:r.seed,
      winner:r.winner_profile,
      lock_steps:r.lock_steps,
      tuned:slot(r,TUNED),
      legacy:slot(r,LEGACY),
    })),
  };
}

const primary=byPace['24'];
let primary_classification='equivocal_or_negative';
if(primary.tuned_wins>=12) primary_classification='strong_confirmation';
else if(primary.tuned_wins>=10) primary_classification='directionally_positive_not_strong';

const robustness={
  pps_2:{
    tuned_wins:byPace['30'].tuned_wins,
    legacy_wins:byPace['30'].legacy_wins,
    non_negative:byPace['30'].tuned_wins>=2,
  },
  pps_3:{
    tuned_wins:byPace['20'].tuned_wins,
    legacy_wins:byPace['20'].legacy_wins,
    non_negative:byPace['20'].tuned_wins>=2,
  },
};
const robustness_non_negative=robustness.pps_2.non_negative && robustness.pps_3.non_negative;
const gate_pass=primary_classification==='strong_confirmation' && robustness_non_negative;

const summary={
  experiment:'tetrp-whole-profile-fresh-confirmation',
  status:'completed',
  authority_ref:rows[0].tetrp_ref,
  node_budget_per_decision:200000,
  design:{
    primary:{
      pps:2.5,
      frames_per_piece:24,
      seeds:primarySeeds,
      independent_seed_count:16,
      preregistered_strong_threshold:'Tuned >= 12 wins out of 16',
    },
    robustness:[
      {pps:2,frames_per_piece:30,seeds:robustness2Seeds},
      {pps:3,frames_per_piece:20,seeds:robustness3Seeds},
    ],
    robustness_rule:'Each robustness block is non-negative if Tuned wins at least 2 of 4. Robustness matches are not added to the primary 16-seed confirmation count.',
  },
  primary_classification,
  robustness_non_negative,
  gate_pass_for_ablation:gate_pass,
  by_pace:byPace,
  interpretation_guard:'Judge confirmation from the 16 fresh 2.5 PPS seeds. The 2/3 PPS blocks are robustness probes only; do not pool all 24 matches into one binomial test or use APP as a tiebreaker.',
};
writeFileSync(out,JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
