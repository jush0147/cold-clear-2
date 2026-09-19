import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root=path.resolve(process.argv[2]||'transfer-results');
const out=path.resolve(process.argv[3]||'transfer-summary.json');

function walk(dir){
  const out=[];
  for(const name of readdirSync(dir,{withFileTypes:true})){
    const p=path.join(dir,name.name);
    if(name.isDirectory()) out.push(...walk(p));
    else if(name.isFile() && name.name.endsWith('.jsonl')) out.push(p);
  }
  return out;
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

const expectedSeeds=[62000,62001,62002,62003,62004,62005,62006,62007];
const expectedFrames=[20,24,30];
const keySet=new Set();
for(const r of rows){
  if(r.node_budget_per_decision!==200000) throw new Error('node budget drift at seed '+r.seed);
  if(!expectedSeeds.includes(r.seed)) throw new Error('unexpected seed '+r.seed);
  if(!expectedFrames.includes(r.frames_per_piece)) throw new Error('unexpected pace '+r.frames_per_piece);
  if(r.synchronous_barrier!==true || r.per_packet_ready_timing!==true) throw new Error('authority contract missing at seed '+r.seed);
  if(r.winner_profile!=='review_h9_h12' && r.winner_profile!=='corrected_legacy_h12') throw new Error('non-unique/unknown winner at seed '+r.seed);
  const k=r.seed+':'+r.frames_per_piece;
  if(keySet.has(k)) throw new Error('duplicate seed/pace '+k);
  keySet.add(k);
}
for(const seed of expectedSeeds) for(const f of expectedFrames) {
  if(!keySet.has(seed+':'+f)) throw new Error('missing seed/pace '+seed+':'+f);
}

const byPace={};
for(const f of expectedFrames){
  const rs=rows.filter(r=>r.frames_per_piece===f).sort((a,b)=>a.seed-b.seed);
  const tuned=rs.filter(r=>r.winner_profile==='review_h9_h12').length;
  const legacy=rs.length-tuned;
  const agg=(profile)=>({
    pieces:rs.reduce((n,r)=>n+r.slots.find(s=>s.profile===profile).pieces,0),
    generated:rs.reduce((n,r)=>n+r.slots.find(s=>s.profile===profile).generated,0),
    sent:rs.reduce((n,r)=>n+r.slots.find(s=>s.profile===profile).sent,0),
  });
  const ta=agg('review_h9_h12'), la=agg('corrected_legacy_h12');
  byPace[String(f)]={
    frames_per_piece:f,
    pps:60/f,
    tuned_wins:tuned,
    legacy_wins:legacy,
    avg_lock_steps:rs.reduce((n,r)=>n+r.lock_steps,0)/rs.length,
    tuned_raw_app:ta.pieces?ta.generated/ta.pieces:0,
    legacy_raw_app:la.pieces?la.generated/la.pieces:0,
    tuned_sent_app:ta.pieces?ta.sent/ta.pieces:0,
    legacy_sent_app:la.pieces?la.sent/la.pieces:0,
    matches:rs.map(r=>({seed:r.seed,winner:r.winner_profile,lock_steps:r.lock_steps}))
  };
}

const perSeed=expectedSeeds.map(seed=>{
  const rs=rows.filter(r=>r.seed===seed).sort((a,b)=>b.frames_per_piece-a.frames_per_piece);
  const tunedWins=rs.filter(r=>r.winner_profile==='review_h9_h12').length;
  return {
    seed,
    tuned_wins:tunedWins,
    legacy_wins:3-tunedWins,
    consistent:tunedWins===0||tunedWins===3,
    pace_reversal:tunedWins>0&&tunedWins<3,
    outcomes:rs.map(r=>({frames_per_piece:r.frames_per_piece,pps:r.pps,winner:r.winner_profile,lock_steps:r.lock_steps}))
  };
});

const tunedTotal=rows.filter(r=>r.winner_profile==='review_h9_h12').length;
const summary={
  experiment:'tetrp-whole-profile-transfer-screen',
  status:'completed',
  authority_ref:rows[0].tetrp_ref,
  node_budget_per_decision:200000,
  unique_piece_seeds:8,
  repeated_paces:[2,2.5,3],
  matches:24,
  tuned_wins:tunedTotal,
  legacy_wins:24-tunedTotal,
  by_pace:byPace,
  per_seed:perSeed,
  pace_reversal_seeds:perSeed.filter(x=>x.pace_reversal).map(x=>x.seed),
  interpretation_guard:'Directional repeated-measures screen only. Do not treat 24 pace-repeated matches as 24 independent Bernoulli trials or use APP as a tiebreaker.'
};
writeFileSync(out,JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
