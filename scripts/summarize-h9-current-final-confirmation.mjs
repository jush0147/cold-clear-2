import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'h9-final-results');
const outputPath=path.resolve(process.argv[3]||'h9-current-final-confirmation-summary.json');
const candidate='reset_h9_cavity_m0_75_h12';
const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';
const expectedPairs=Array.from({length:200},(_,i)=>[65500+i*2,65501+i*2]);
const expectedKeys=new Set(expectedPairs.map(p=>p.join('-')));

function walk(dir){
  return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>{
    const p=path.join(dir,e.name);
    return e.isDirectory()?walk(p):[p];
  });
}
function choose(n,k){k=Math.min(k,n-k);let x=1;for(let i=1;i<=k;i++)x=x*(n-k+i)/i;return x;}
function signP(a,b){
  const n=a+b;if(n===0)return 1;
  const k=Math.min(a,b);let p=0;
  for(let i=0;i<=k;i++)p+=choose(n,i)/2**n;
  return Math.min(1,2*p);
}
function wilson(success,total,z=1.959963984540054){
  if(total===0)return [null,null];
  const phat=success/total;
  const d=1+z*z/total;
  const center=(phat+z*z/(2*total))/d;
  const half=z*Math.sqrt(phat*(1-phat)/total+z*z/(4*total*total))/d;
  return [center-half,center+half];
}

const rows=[];
for(const file of walk(inputDir)){
  if(!file.endsWith('.jsonl'))continue;
  for(const [i,line] of fs.readFileSync(file,'utf8').split(/\r?\n/).entries()){
    if(!line.trim())continue;
    let r;
    try{r=JSON.parse(line);}catch(e){throw new Error(`invalid JSON ${file}:${i+1}: ${e.message}`);}
    if(r.type==='tetrp_synchronous_match'||r.type==='tetrp_reset_screen_failure')rows.push(r);
  }
}

const relevant=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===candidate);
const failures=relevant.filter(r=>r.type==='tetrp_reset_screen_failure');
const games=relevant.filter(r=>r.type==='tetrp_synchronous_match');
const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));

let cp=0,cg=0,cs=0,ip=0,ig=0,is=0;
for(const g of games){
  const meta=g.screen_pair;
  if(!meta)throw new Error('missing screen_pair');
  const key=meta.pair_seeds?.join('-');
  if(!expectedKeys.has(key))throw new Error('unexpected pair '+key);
  if(g.node_budget_per_decision!==200000||g.frames_per_piece!==24||g.tetrp_ref!==tetrpRef)throw new Error('protocol mismatch');
  if(g.scored!==true||g.diagnostic_stop_reason!==null)throw new Error('unscored game entered H9 final confirmation');
  if(g.profiles[meta.candidate_slot]!==candidate||g.profiles[1-meta.candidate_slot]!==incumbent)throw new Error('profile-slot mismatch');
  byPair.get(key).push(g);
  const c=g.slots.find(x=>x.profile===candidate);
  const b=g.slots.find(x=>x.profile===incumbent);
  if(!c||!b)throw new Error('slot diagnostics mismatch');
  cp+=c.pieces;cg+=c.generated;cs+=c.sent;
  ip+=b.pieces;ig+=b.generated;is+=b.sent;
}

let candidateSweeps=0,incumbentSweeps=0,splitPairs=0;
const pairProblems=[];
for(const p of expectedPairs){
  const gs=byPair.get(p.join('-'));
  const slots=gs.map(g=>g.screen_pair.candidate_slot).sort();
  if(gs.length!==2||slots[0]!==0||slots[1]!==1){
    pairProblems.push({pair:p,count:gs.length,candidate_slots:slots});
    continue;
  }
  const wins=gs.filter(g=>g.winner_profile===candidate).length;
  if(wins===2)candidateSweeps++;
  else if(wins===0)incumbentSweeps++;
  else splitPairs++;
}

const candidateWins=games.filter(g=>g.winner_profile===candidate).length;
const incumbentWins=games.filter(g=>g.winner_profile===incumbent).length;
const p=signP(candidateSweeps,incumbentSweeps);
const complete=failures.length===0&&games.length===400&&pairProblems.length===0;
const decisive=candidateSweeps+incumbentSweeps;
const [sweepWilsonLow,sweepWilsonHigh]=wilson(candidateSweeps,decisive);
const promote=complete&&candidateSweeps>incumbentSweeps&&p<=0.05&&candidateWins>200;

const out={
  experiment:'H9 current-stack fixed-finalist confirmation',
  all_complete:complete,
  protocol:{
    finalist_value:-0.75,
    candidate_profile:candidate,
    incumbent_profile:incumbent,
    node_budget_per_decision:200000,
    frames_per_piece:24,
    pps:2.5,
    paired_units:200,
    total_games:400,
    promotion_authority:true
  },
  ko:{candidate_wins:candidateWins,incumbent_wins:incumbentWins},
  pairs:{
    candidate_sweeps:candidateSweeps,
    incumbent_sweeps:incumbentSweeps,
    split_pairs:splitPairs,
    decisive_pairs:decisive,
    net_sweeps:candidateSweeps-incumbentSweeps,
    exact_two_sided_sign_p:p,
    candidate_sweep_share_among_decisive:decisive?candidateSweeps/decisive:null,
    candidate_sweep_share_wilson95:[sweepWilsonLow,sweepWilsonHigh]
  },
  diagnostics:{
    candidate:{raw_app:cp?cg/cp:null,sent_app:cp?cs/cp:null,pieces:cp},
    incumbent:{raw_app:ip?ig/ip:null,sent_app:ip?is/ip:null,pieces:ip}
  },
  integrity:{
    failures:failures.map(f=>({pair_seeds:f.pair_seeds,error:f.error})),
    pair_problems:pairProblems
  },
  decision:!complete?{
    action:'blocked',
    reason:'Confirmation is incomplete or contains a harness/integrity failure.'
  }:promote?{
    action:'promote_h9',
    new_incumbent_h9_cavity_excavation:-0.75,
    reason:'The fixed finalist passed every preregistered promotion criterion.'
  }:{
    action:'do_not_promote_h9',
    reason:'The fixed finalist did not pass every preregistered promotion criterion.'
  }
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(out,null,2)+'\n');
process.stdout.write(JSON.stringify(out,null,2)+'\n');
if(!complete)process.exitCode=2;
