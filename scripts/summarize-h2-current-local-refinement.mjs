import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'h2-local-results');
const outputPath=path.resolve(process.argv[3]||'h2-current-local-refinement-summary.json');
const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';
const defs=[
  ['a1_c0',1.0,0.0],
  ['a1_25_c0',1.25,0.0],
  ['a1_5_c0',1.5,0.0],
  ['a1_5_c0_25',1.5,0.25],
  ['a1_5_c0_5',1.5,0.5],
  ['a1_75_c0_5',1.75,0.5],
];
const configs=defs.map(([id,attack,cancel])=>({
  id,attack,cancel,
  profile:'tuner:'+JSON.stringify({useful_attack_reward:attack,cancellation_reward:cancel}),
}));
const expectedPairs=Array.from({length:40},(_,i)=>[66000+i*2,66001+i*2]);
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

const results=[];
for(const cfg of configs){
  const rs=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===cfg.profile);
  const failures=rs.filter(r=>r.type==='tetrp_reset_screen_failure');
  const games=rs.filter(r=>r.type==='tetrp_synchronous_match');
  const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));
  let cp=0,cg=0,cs=0,ip=0,ig=0,is=0;

  for(const g of games){
    const meta=g.screen_pair;
    if(!meta)throw new Error('missing screen_pair');
    const key=meta.pair_seeds?.join('-');
    if(!expectedKeys.has(key))throw new Error(`unexpected pair for ${cfg.id}: ${key}`);
    if(g.node_budget_per_decision!==200000||g.frames_per_piece!==24||g.tetrp_ref!==tetrpRef)throw new Error('protocol mismatch');
    if(g.scored!==true||g.diagnostic_stop_reason!==null)throw new Error('unscored game entered H2 local refinement');
    if(g.profiles[meta.candidate_slot]!==cfg.profile||g.profiles[1-meta.candidate_slot]!==incumbent)throw new Error('profile-slot mismatch');
    byPair.get(key).push(g);
    const c=g.slots.find(x=>x.profile===cfg.profile);
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
    const wins=gs.filter(g=>g.winner_profile===cfg.profile).length;
    if(wins===2)candidateSweeps++;
    else if(wins===0)incumbentSweeps++;
    else splitPairs++;
  }

  const candidateWins=games.filter(g=>g.winner_profile===cfg.profile).length;
  const incumbentWins=games.filter(g=>g.winner_profile===incumbent).length;
  const complete=failures.length===0&&games.length===80&&pairProblems.length===0;
  const eligible=complete&&candidateSweeps>incumbentSweeps&&candidateWins>40;
  results.push({
    id:cfg.id,
    useful_attack_reward:cfg.attack,
    cancellation_reward:cfg.cancel,
    profile:cfg.profile,
    complete,
    eligible,
    ko:{candidate_wins:candidateWins,incumbent_wins:incumbentWins},
    pairs:{
      candidate_sweeps:candidateSweeps,
      incumbent_sweeps:incumbentSweeps,
      split_pairs:splitPairs,
      decisive_pairs:candidateSweeps+incumbentSweeps,
      net_sweeps:candidateSweeps-incumbentSweeps,
      exact_two_sided_sign_p:signP(candidateSweeps,incumbentSweeps),
    },
    diagnostics:{
      candidate:{raw_app:cp?cg/cp:null,sent_app:cp?cs/cp:null,pieces:cp},
      incumbent:{raw_app:ip?ig/ip:null,sent_app:ip?is/ip:null,pieces:ip},
    },
    failures:failures.map(f=>({pair_seeds:f.screen_pair?.pair_seeds??f.pair_seeds,error:f.error})),
    pair_problems:pairProblems,
  });
}

const allComplete=results.every(x=>x.complete);
const eligible=results.filter(x=>x.eligible).sort((a,b)=>
  b.pairs.net_sweeps-a.pairs.net_sweeps ||
  b.ko.candidate_wins-a.ko.candidate_wins
);

let decision={action:'blocked',reason:'Integrity incomplete.'};
if(allComplete){
  if(eligible.length===0){
    decision={action:'no_h2_finalist',reason:'No refined candidate met both preregistered eligibility criteria.'};
  }else{
    const top=eligible[0];
    const tied=eligible.filter(x=>x.pairs.net_sweeps===top.pairs.net_sweeps&&x.ko.candidate_wins===top.ko.candidate_wins);
    if(tied.length>1){
      decision={action:'fresh_decider_required',tied_ids:tied.map(x=>x.id),reason:'Top candidates tied on both preregistered ranking metrics.'};
    }else if(top.id==='a1_75_c0_5'){
      decision={action:'extend_attack_boundary',leader_id:top.id,reason:'The unique top refined candidate is the preregistered outward useful-attack boundary point.'};
    }else{
      decision={action:'select_fixed_finalist',finalist_id:top.id,finalist_profile:top.profile,useful_attack_reward:top.useful_attack_reward,cancellation_reward:top.cancellation_reward,reason:'Unique top-ranked eligible candidate under the preregistered local-refinement rule.'};
    }
  }
}

const out={
  experiment:'H2 current-stack local refinement',
  all_complete:allComplete,
  protocol:{
    incumbent_profile:incumbent,
    node_budget_per_decision:200000,
    frames_per_piece:24,
    pps:2.5,
    paired_units_per_candidate:40,
    games_per_candidate:80,
    authority_seed_pairs:expectedPairs,
    promotion_authority:false,
  },
  results,
  decision,
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(out,null,2)+'\n');
process.stdout.write(JSON.stringify(out,null,2)+'\n');
if(!allComplete)process.exitCode=2;
