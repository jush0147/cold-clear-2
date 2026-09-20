import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'h9-local-results');
const outputPath=path.resolve(process.argv[3]||'h9-current-local-refinement-summary.json');
const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';
const configs=[
  {id:'m0_625',value:-0.625,profile:'reset_h9_cavity_m0_625_h12'},
  {id:'m0_5',value:-0.5,profile:'reset_h9_cavity_m0_5_h12'},
  {id:'m0_375',value:-0.375,profile:'reset_h9_cavity_m0_375_h12'},
  {id:'m0_25',value:-0.25,profile:'reset_h9_cavity_m0_25_h12'},
  {id:'m0_125',value:-0.125,profile:'reset_h9_cavity_m0_125_h12'},
];
const expectedPairs=Array.from({length:40},(_,i)=>[65300+i*2,65301+i*2]);
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

const summaries=[];
for(const cfg of configs){
  const rs=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===cfg.profile);
  const failures=rs.filter(r=>r.type==='tetrp_reset_screen_failure');
  const games=rs.filter(r=>r.type==='tetrp_synchronous_match');
  const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));
  for(const g of games){
    const meta=g.screen_pair;
    if(!meta)throw new Error('missing screen_pair');
    const key=meta.pair_seeds?.join('-');
    if(!expectedKeys.has(key))throw new Error(`unexpected pair for ${cfg.id}: ${key}`);
    if(g.node_budget_per_decision!==200000||g.frames_per_piece!==24||g.tetrp_ref!==tetrpRef)throw new Error('protocol mismatch');
    if(g.scored!==true||g.diagnostic_stop_reason!==null)throw new Error('unscored game entered H9 local refinement');
    if(g.profiles[meta.candidate_slot]!==cfg.profile||g.profiles[1-meta.candidate_slot]!==incumbent)throw new Error('profile-slot mismatch');
    byPair.get(key).push(g);
  }

  let candidateSweeps=0,incumbentSweeps=0,splitPairs=0;
  const pairProblems=[];
  for(const p of expectedPairs){
    const a=byPair.get(p.join('-'));
    const slots=a.map(x=>x.screen_pair.candidate_slot).sort();
    if(a.length!==2||slots[0]!==0||slots[1]!==1){
      pairProblems.push({pair:p,count:a.length,candidate_slots:slots});
      continue;
    }
    const cw=a.filter(x=>x.winner_profile===cfg.profile).length;
    if(cw===2)candidateSweeps++;
    else if(cw===0)incumbentSweeps++;
    else splitPairs++;
  }

  const candidateWins=games.filter(g=>g.winner_profile===cfg.profile).length;
  const incumbentWins=games.filter(g=>g.winner_profile===incumbent).length;
  const complete=failures.length===0&&games.length===80&&pairProblems.length===0;
  let cp=0,cg=0,cs=0,ip=0,ig=0,is=0;
  for(const g of games){
    const c=g.slots.find(x=>x.profile===cfg.profile);
    const b=g.slots.find(x=>x.profile===incumbent);
    if(!c||!b)throw new Error('slot diagnostics mismatch');
    cp+=c.pieces;cg+=c.generated;cs+=c.sent;
    ip+=b.pieces;ig+=b.generated;is+=b.sent;
  }
  summaries.push({
    id:cfg.id,value:cfg.value,profile:cfg.profile,complete,
    ko:{candidate_wins:candidateWins,incumbent_wins:incumbentWins},
    pairs:{
      candidate_sweeps:candidateSweeps,
      incumbent_sweeps:incumbentSweeps,
      split_pairs:splitPairs,
      net_sweeps:candidateSweeps-incumbentSweeps,
      exact_two_sided_sign_p:signP(candidateSweeps,incumbentSweeps),
    },
    eligible_for_finalist:complete&&candidateSweeps>incumbentSweeps&&candidateWins>40,
    diagnostics:{
      candidate:{raw_app:cp?cg/cp:null,sent_app:cp?cs/cp:null,pieces:cp},
      incumbent:{raw_app:ip?ig/ip:null,sent_app:ip?is/ip:null,pieces:ip},
    },
    failures:failures.map(f=>({pair_seeds:f.pair_seeds,error:f.error})),
    pair_problems:pairProblems,
  });
}

const allComplete=summaries.every(x=>x.complete);
const eligible=summaries.filter(x=>x.eligible_for_finalist).sort((a,b)=>
  b.pairs.net_sweeps-a.pairs.net_sweeps ||
  b.ko.candidate_wins-a.ko.candidate_wins ||
  Math.abs(a.value+0.375)-Math.abs(b.value+0.375) ||
  Math.abs(a.value)-Math.abs(b.value)
);
const best=eligible[0]??null;
const boundary=best&&(best.value===-0.625||best.value===-0.125);
const selection=!allComplete?{
  action:'blocked',
  reason:'At least one refinement value is incomplete; do not select a finalist.'
}:!best?{
  action:'pause_h9',
  reason:'No refinement value met the preregistered positive-direction eligibility rule.'
}:boundary?{
  action:'extend_boundary',
  best_observed_value:best.value,
  reason:'The best eligible value is on the preregistered refinement boundary; extend outward with fresh seeds before fixing a finalist.'
}:{
  action:'select_fixed_finalist',
  finalist_value:best.value,
  finalist_profile:best.profile,
  reason:'Highest preregistered net paired-sweep result among eligible interior values, with preregistered tie-breakers.'
};

const out={
  experiment:'H9 current-stack local refinement',
  all_complete:allComplete,
  protocol:{
    incumbent_profile:incumbent,
    node_budget_per_decision:200000,
    frames_per_piece:24,
    pps:2.5,
    paired_units_per_value:40,
    games_per_value:80,
    authority_seed_pairs:expectedPairs,
    promotion_authority:false
  },
  results:summaries,
  selection
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(out,null,2)+'\n');
process.stdout.write(JSON.stringify(out,null,2)+'\n');
if(!allComplete)process.exitCode=2;
