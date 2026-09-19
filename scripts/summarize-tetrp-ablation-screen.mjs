import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root=path.resolve(process.argv[2]||'ablation-results');
const out=path.resolve(process.argv[3]||'ablation-summary.json');

const FULL='review_h9_h12';
const seeds=Array.from({length:8},(_,i)=>64000+i);
const variants=[
  {name:'minus_h1', profile:'review_minus_h1_h12'},
  {name:'minus_h2', profile:'review_minus_h2_h12'},
  {name:'minus_h6c', profile:'review_minus_h6c_h12'},
  {name:'minus_h9', profile:'review_minus_h9_h12'},
];

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
  if(!s) throw new Error('missing profile '+profile+' at seed '+r.seed);
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
function classify(fullWins){
  if(fullWins>=6) return 'retain_signal';
  if(fullWins===5) return 'weak_retain_signal';
  if(fullWins===4) return 'inconclusive';
  return 'removal_not_worse_signal';
}

const rows=[];
for(const file of walk(root)){
  for(const line of readFileSync(file,'utf8').split(/\r?\n/)){
    if(!line.trim()) continue;
    const row=JSON.parse(line);
    if(row.type==='tetrp_synchronous_match') rows.push(row);
  }
}
if(rows.length!==32) throw new Error('expected 32 scored matches, found '+rows.length);

const seen=new Set();
for(const r of rows){
  if(r.node_budget_per_decision!==200000) throw new Error('node budget drift at seed '+r.seed);
  if(r.frames_per_piece!==24) throw new Error('unexpected pace at seed '+r.seed+': '+r.frames_per_piece);
  if(!seeds.includes(r.seed)) throw new Error('unexpected seed '+r.seed);
  if(r.synchronous_barrier!==true || r.per_packet_ready_timing!==true || r.zero_gravity!==true) {
    throw new Error('authority contract missing at seed '+r.seed);
  }
  if(!r.profiles.includes(FULL)) throw new Error('full profile missing at seed '+r.seed);
  const variant=variants.find(v=>r.profiles.includes(v.profile));
  if(!variant) throw new Error('unknown/missing ablation profile at seed '+r.seed);
  if(r.winner_profile!==FULL && r.winner_profile!==variant.profile) {
    throw new Error('non-unique/unknown winner at seed '+r.seed+' for '+variant.name);
  }
  const key=variant.name+':'+r.seed;
  if(seen.has(key)) throw new Error('duplicate ablation/seed '+key);
  seen.add(key);
}
for(const v of variants) for(const seed of seeds) {
  const key=v.name+':'+seed;
  if(!seen.has(key)) throw new Error('missing ablation/seed '+key);
}

const byAblation={};
for(const v of variants){
  const rs=rows.filter(r=>r.profiles.includes(v.profile)).sort((a,b)=>a.seed-b.seed);
  const fullWins=rs.filter(r=>r.winner_profile===FULL).length;
  const ablatedWins=rs.length-fullWins;
  const locks=rs.map(r=>r.lock_steps);
  byAblation[v.name]={
    ablated_profile:v.profile,
    seeds,
    matches:rs.length,
    full_wins:fullWins,
    ablated_wins:ablatedWins,
    classification:classify(fullWins),
    lock_steps:{
      avg:locks.reduce((a,b)=>a+b,0)/locks.length,
      median:median(locks),
      min:Math.min(...locks),
      max:Math.max(...locks),
      values:rs.map(r=>({seed:r.seed,lock_steps:r.lock_steps})),
    },
    full:aggregate(rs,FULL),
    ablated:aggregate(rs,v.profile),
    per_match:rs.map(r=>({
      seed:r.seed,
      winner:r.winner_profile,
      lock_steps:r.lock_steps,
      full:slot(r,FULL),
      ablated:slot(r,v.profile),
    })),
  };
}

const summary={
  experiment:'tetrp-single-component-ablation-screen',
  status:'completed',
  authority_ref:rows[0].tetrp_ref,
  node_budget_per_decision:200000,
  pps:2.5,
  frames_per_piece:24,
  seeds,
  matches_per_ablation:8,
  by_ablation:byAblation,
  followup_needed:Object.entries(byAblation)
    .filter(([,x])=>x.classification!=='retain_signal')
    .map(([name,x])=>({name,classification:x.classification,score:x.full_wins+'-'+x.ablated_wins})),
  interpretation_guard:'Each ablation is its own 8-seed screen. Do not pool all 32 matches into a single feature-strength claim, and do not use APP to override KO outcomes.',
};
writeFileSync(out,JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary,null,2));
