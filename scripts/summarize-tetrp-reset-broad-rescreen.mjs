import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'reset-screen-results');
const outputPath=path.resolve(process.argv[3]||'reset-screen-summary.json');

const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';
const candidates=[
  ['h1','reset_h1_pending_safety_1_h12'],
  ['h2','reset_h2_useful_attack_1_h12'],
  ['h3','reset_h3_b2b_charge_1_5_h12'],
  ['h4','reset_h4_well_half_h12'],
  ['h5','reset_h5_combo_4_h12'],
  ['h6','reset_h6_height_0_75_h12'],
  ['h6b','reset_h6b_holes1_5_covered0_5_h12'],
  ['h6c','reset_h6c_row2_5_h12'],
  ['h9','reset_h9_cavity_m0_5_h12'],
  ['h10','reset_h10_exploitation_0_7985_h12'],
  ['h11','reset_h11_k40s60_h12'],
];
const expectedPairs=[
  [65000,65001],[65002,65003],[65004,65005],[65006,65007],
  [65008,65009],[65010,65011],[65012,65013],[65014,65015],
];

function walk(dir){
  return fs.readdirSync(dir,{withFileTypes:true}).flatMap(entry=>{
    const p=path.join(dir,entry.name);
    return entry.isDirectory()?walk(p):[p];
  });
}
function pairKey(pair){return pair.join('-');}
function mean(xs){return xs.length?xs.reduce((a,b)=>a+b,0)/xs.length:null;}
function median(xs){
  if(!xs.length)return null;
  const a=[...xs].sort((x,y)=>x-y);
  const m=Math.floor(a.length/2);
  return a.length%2?a[m]:(a[m-1]+a[m])/2;
}
function choose(n,k){
  k=Math.min(k,n-k);
  let x=1;
  for(let i=1;i<=k;i++)x=x*(n-k+i)/i;
  return x;
}
function exactTwoSidedSignP(a,b){
  const n=a+b;
  if(n===0)return 1;
  const k=Math.min(a,b);
  let lower=0;
  for(let i=0;i<=k;i++)lower+=choose(n,i)/2**n;
  return Math.min(1,2*lower);
}

const records=[];
for(const file of walk(inputDir)){
  if(!file.endsWith('.jsonl'))continue;
  for(const [lineNo,line] of fs.readFileSync(file,'utf8').split(/\r?\n/).entries()){
    if(!line.trim())continue;
    let row;
    try{row=JSON.parse(line);}catch(e){
      throw new Error(`invalid JSON in ${file}:${lineNo+1}: ${e.message}`);
    }
    if(row.type==='tetrp_synchronous_match' || row.type==='tetrp_reset_screen_failure'){
      records.push({...row,_file:file,_line:lineNo+1});
    }
  }
}

const expectedPairKeys=new Set(expectedPairs.map(pairKey));
const byProfile=new Map(candidates.map(([,p])=>[p,{games:[],failures:[]}]));
for(const row of records){
  const profile=row.screen_pair?.candidate_profile ?? row.candidate_profile;
  if(!byProfile.has(profile))continue;
  if(row.type==='tetrp_reset_screen_failure'){
    byProfile.get(profile).failures.push(row);
    continue;
  }

  const meta=row.screen_pair;
  if(!meta)throw new Error('scored reset-screen game missing screen_pair metadata');
  const pair=meta.pair_seeds;
  if(!Array.isArray(pair)||pair.length!==2||!expectedPairKeys.has(pairKey(pair))){
    throw new Error(`unexpected pair for ${profile}: ${JSON.stringify(pair)}`);
  }
  if(row.node_budget_per_decision!==200000)throw new Error('wrong node budget');
  if(row.frames_per_piece!==24)throw new Error('wrong frames_per_piece');
  if(row.tetrp_ref!==tetrpRef)throw new Error('wrong Tetrp ref');
  if(row.zero_gravity!==true||row.synchronous_barrier!==true||row.per_packet_ready_timing!==true){
    throw new Error('authority invariant missing');
  }
  if(row.scored!==true||row.diagnostic_stop_reason!==null){
    throw new Error('diagnostic/unscored row entered scored screen');
  }
  if(!Array.isArray(row.seeds)||row.seeds.length!==2||
     row.seeds[0]!==pair[0]||row.seeds[1]!==pair[1]){
    throw new Error('authority seed pair mismatch');
  }
  if(!Array.isArray(row.profiles)||row.profiles.length!==2||
     row.profiles[meta.candidate_slot]!==profile||
     row.profiles[1-meta.candidate_slot]!==incumbent){
    throw new Error('candidate/incumbent slot metadata mismatch');
  }
  if(meta.candidate_seed!==row.seeds[meta.candidate_slot]||
     meta.incumbent_seed!==row.seeds[1-meta.candidate_slot]){
    throw new Error('candidate/incumbent seed assignment mismatch');
  }
  if(row.winner_profile!==profile && row.winner_profile!==incumbent){
    throw new Error('match has no valid KO winner profile');
  }
  byProfile.get(profile).games.push(row);
}

const summaries=[];
for(const [id,profile] of candidates){
  const {games,failures}=byProfile.get(profile);
  const perPair=new Map(expectedPairs.map(p=>[pairKey(p),[]]));
  for(const g of games)perPair.get(pairKey(g.screen_pair.pair_seeds)).push(g);

  const pairProblems=[];
  let candidateSweeps=0, incumbentSweeps=0, splitPairs=0;
  for(const p of expectedPairs){
    const rows=perPair.get(pairKey(p));
    const slots=rows.map(r=>r.screen_pair.candidate_slot).sort();
    if(rows.length!==2||slots[0]!==0||slots[1]!==1){
      pairProblems.push({pair:p,scored_games:rows.length,candidate_slots:slots});
      continue;
    }
    const cw=rows.filter(r=>r.winner_profile===profile).length;
    if(cw===2)candidateSweeps++;
    else if(cw===0)incumbentSweeps++;
    else splitPairs++;
  }

  const candidateWins=games.filter(g=>g.winner_profile===profile).length;
  const incumbentWins=games.filter(g=>g.winner_profile===incumbent).length;
  const complete=failures.length===0&&games.length===16&&pairProblems.length===0;

  let classification='incomplete_unscored';
  if(complete){
    if(candidateWins>=11)classification='screened_positive';
    else if(candidateWins<=5)classification='screened_negative';
    else classification='screened_neutral';
  }
  const direction=!complete?'unknown':candidateWins>8?'positive':candidateWins<8?'negative':'flat';
  const extensionEligible=complete&&classification==='screened_neutral'&&candidateWins!==8;

  let candPieces=0,candGenerated=0,candSent=0,candCancelled=0,candTanked=0,candReceived=0;
  let incPieces=0,incGenerated=0,incSent=0,incCancelled=0,incTanked=0,incReceived=0;
  for(const g of games){
    const cs=g.slots.find(s=>s.profile===profile);
    const is=g.slots.find(s=>s.profile===incumbent);
    if(!cs||!is)throw new Error('slot summary profile mismatch');
    candPieces+=cs.pieces; candGenerated+=cs.generated; candSent+=cs.sent;
    candCancelled+=cs.cancelled; candTanked+=cs.tanked; candReceived+=cs.received;
    incPieces+=is.pieces; incGenerated+=is.generated; incSent+=is.sent;
    incCancelled+=is.cancelled; incTanked+=is.tanked; incReceived+=is.received;
  }

  const locks=games.map(g=>g.lock_steps);
  summaries.push({
    id,profile,incumbent_profile:incumbent,
    complete,
    classification,
    direction,
    extension_eligible:extensionEligible,
    scored_games:games.length,
    failures:failures.map(f=>({
      pair_seeds:f.pair_seeds,
      candidate_slot:f.candidate_slot,
      error:f.error,
    })),
    pair_problems:pairProblems,
    ko:{
      candidate_wins:candidateWins,
      incumbent_wins:incumbentWins,
      candidate_sweeps:candidateSweeps,
      incumbent_sweeps:incumbentSweeps,
      split_pairs:splitPairs,
      paired_sign_p:exactTwoSidedSignP(candidateSweeps,incumbentSweeps),
    },
    lock_steps:{
      average:mean(locks),
      median:median(locks),
      min:locks.length?Math.min(...locks):null,
      max:locks.length?Math.max(...locks):null,
      per_game:games.map(g=>({
        pair_seeds:g.screen_pair.pair_seeds,
        candidate_slot:g.screen_pair.candidate_slot,
        candidate_seed:g.screen_pair.candidate_seed,
        winner_profile:g.winner_profile,
        lock_steps:g.lock_steps,
      })),
    },
    diagnostics:{
      candidate:{
        pieces:candPieces,
        generated:candGenerated,
        cancelled:candCancelled,
        sent:candSent,
        tanked:candTanked,
        received:candReceived,
        raw_app:candPieces?candGenerated/candPieces:null,
        sent_app:candPieces?candSent/candPieces:null,
      },
      incumbent:{
        pieces:incPieces,
        generated:incGenerated,
        cancelled:incCancelled,
        sent:incSent,
        tanked:incTanked,
        received:incReceived,
        raw_app:incPieces?incGenerated/incPieces:null,
        sent_app:incPieces?incSent/incPieces:null,
      },
    },
  });
}

const allComplete=summaries.every(x=>x.complete);
const out={
  experiment:'Tetrp current-stack broad strategy rescreen',
  protocol:{
    incumbent_profile:incumbent,
    node_budget_per_decision:200000,
    frames_per_piece:24,
    pps:2.5,
    paired_units_per_candidate:8,
    games_per_candidate:16,
    authority_seed_pairs:expectedPairs,
    result_rule:'KO only',
    classification_rule:{
      screened_positive:'candidate wins >=11/16',
      screened_negative:'candidate wins <=5/16',
      screened_neutral:'candidate wins 6-10/16',
    },
    guard:'The eight asymmetric seed pairs are the experimental units. Do not treat 16 games as independent, do not pool candidate families, and do not let APP override KO.',
  },
  all_complete:allComplete,
  candidates:summaries,
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(out,null,2)+'\n');
process.stdout.write(JSON.stringify(out,null,2)+'\n');
if(!allComplete)process.exitCode=2;
