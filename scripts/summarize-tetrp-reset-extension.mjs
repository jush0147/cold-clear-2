import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'reset-extension-results');
const outputPath=path.resolve(process.argv[3]||'reset-extension-summary.json');
const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';

const candidates=[
  {id:'h2',profile:'reset_h2_useful_attack_1_h12',oldWins:10,oldLosses:6,oldCS:3,oldIS:1,oldSplit:4},
  {id:'h4',profile:'reset_h4_well_half_h12',oldWins:9,oldLosses:7,oldCS:3,oldIS:2,oldSplit:3},
  {id:'h6',profile:'reset_h6_height_0_75_h12',oldWins:9,oldLosses:7,oldCS:2,oldIS:1,oldSplit:5},
  {id:'h6c',profile:'reset_h6c_row2_5_h12',oldWins:10,oldLosses:6,oldCS:4,oldIS:2,oldSplit:2},
  {id:'h9',profile:'reset_h9_cavity_m0_5_h12',oldWins:10,oldLosses:6,oldCS:3,oldIS:1,oldSplit:4},
  {id:'h10',profile:'reset_h10_exploitation_0_7985_h12',oldWins:9,oldLosses:7,oldCS:4,oldIS:3,oldSplit:1},
];
const expectedPairs=[[65016,65017],[65018,65019],[65020,65021],[65022,65023]];
const expectedKeys=new Set(expectedPairs.map(p=>p.join('-')));

function walk(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(dir,e.name)):[path.join(dir,e.name)]);}
function choose(n,k){k=Math.min(k,n-k);let x=1;for(let i=1;i<=k;i++)x=x*(n-k+i)/i;return x;}
function signP(a,b){const n=a+b;if(n===0)return 1;const k=Math.min(a,b);let p=0;for(let i=0;i<=k;i++)p+=choose(n,i)/2**n;return Math.min(1,2*p);}
const rows=[];
for(const f of walk(inputDir)){
  if(!f.endsWith('.jsonl'))continue;
  for(const line of fs.readFileSync(f,'utf8').split(/\r?\n/)){
    if(!line.trim())continue;
    const r=JSON.parse(line);
    if(r.type==='tetrp_synchronous_match'||r.type==='tetrp_reset_screen_failure')rows.push(r);
  }
}
const out=[];
for(const c of candidates){
  const rs=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===c.profile);
  const failures=rs.filter(r=>r.type==='tetrp_reset_screen_failure');
  const games=rs.filter(r=>r.type==='tetrp_synchronous_match');
  const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));
  for(const g of games){
    const k=g.screen_pair?.pair_seeds?.join('-');
    if(!expectedKeys.has(k))throw new Error('unexpected pair '+k);
    if(g.node_budget_per_decision!==200000||g.frames_per_piece!==24||g.tetrp_ref!==tetrpRef)throw new Error('protocol mismatch');
    byPair.get(k).push(g);
  }
  let cs=0,is=0,sp=0; const probs=[];
  for(const p of expectedPairs){
    const a=byPair.get(p.join('-'));
    const slots=a.map(x=>x.screen_pair.candidate_slot).sort();
    if(a.length!==2||slots[0]!==0||slots[1]!==1){probs.push({pair:p,count:a.length,slots});continue;}
    const w=a.filter(x=>x.winner_profile===c.profile).length;
    if(w===2)cs++; else if(w===0)is++; else sp++;
  }
  const nw=games.filter(g=>g.winner_profile===c.profile).length;
  const nl=games.filter(g=>g.winner_profile===incumbent).length;
  const combinedWins=c.oldWins+nw, combinedLosses=c.oldLosses+nl;
  const combinedCS=c.oldCS+cs, combinedIS=c.oldIS+is, combinedSplit=c.oldSplit+sp;
  const complete=failures.length===0&&games.length===8&&probs.length===0;
  let classification='incomplete_unscored';
  if(complete){
    classification=combinedWins>=15?'screened_positive_after_extension':combinedWins<=9?'screened_negative_after_extension':'screened_neutral_after_extension';
  }
  out.push({
    id:c.id,profile:c.profile,complete,classification,
    extension:{candidate_wins:nw,incumbent_wins:nl,candidate_sweeps:cs,incumbent_sweeps:is,split_pairs:sp},
    combined:{candidate_wins:combinedWins,incumbent_wins:combinedLosses,candidate_sweeps:combinedCS,incumbent_sweeps:combinedIS,split_pairs:combinedSplit,paired_sign_p:signP(combinedCS,combinedIS)},
    failures:failures.map(f=>({pair_seeds:f.pair_seeds,error:f.error})),
    pair_problems:probs,
  });
}
const allComplete=out.every(x=>x.complete);
const summary={
  experiment:'Tetrp reset positive-neutral extension',
  protocol:{old_pairs:8,new_pairs:4,total_pairs:12,total_games:24,node_budget_per_decision:200000,frames_per_piece:24,new_seed_pairs:expectedPairs,
    rule:{positive:'>=15/24',negative:'<=9/24',neutral:'10-14/24'},guard:'KO only; APP cannot override; 12 asymmetric pairs are the experimental units.'},
  all_complete:allComplete,candidates:out
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(summary,null,2)+'\n');
process.stdout.write(JSON.stringify(summary,null,2)+'\n');
if(!allComplete)process.exitCode=2;
