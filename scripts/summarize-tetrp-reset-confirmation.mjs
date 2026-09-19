import fs from 'node:fs';
import path from 'node:path';

const inputDir=path.resolve(process.argv[2]||'reset-confirmation-results');
const outputPath=path.resolve(process.argv[3]||'reset-confirmation-summary.json');
const incumbent='corrected_legacy_h12';
const tetrpRef='0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2';
const candidates=[
  {id:'h9',profile:'reset_h9_cavity_m0_5_h12'},
  {id:'h4',profile:'reset_h4_well_half_h12'},
  {id:'h2',profile:'reset_h2_useful_attack_1_h12'},
];
const expectedPairs=Array.from({length:16},(_,i)=>[65100+i*2,65101+i*2]);
const expectedKeys=new Set(expectedPairs.map(p=>p.join('-')));

function walk(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(dir,e.name)):[path.join(dir,e.name)]);}
function choose(n,k){k=Math.min(k,n-k);let x=1;for(let i=1;i<=k;i++)x=x*(n-k+i)/i;return x;}
function signP(a,b){const n=a+b;if(n===0)return 1;const k=Math.min(a,b);let p=0;for(let i=0;i<=k;i++)p+=choose(n,i)/2**n;return Math.min(1,2*p);}
function mean(xs){return xs.length?xs.reduce((a,b)=>a+b,0)/xs.length:null;}
function median(xs){if(!xs.length)return null;const a=[...xs].sort((x,y)=>x-y),m=Math.floor(a.length/2);return a.length%2?a[m]:(a[m-1]+a[m])/2;}

const rows=[];
for(const f of walk(inputDir)){
  if(!f.endsWith('.jsonl'))continue;
  for(const line of fs.readFileSync(f,'utf8').split(/\r?\n/)){
    if(!line.trim())continue;
    const r=JSON.parse(line);
    if(r.type==='tetrp_synchronous_match'||r.type==='tetrp_reset_screen_failure')rows.push(r);
  }
}

const summaries=[];
for(const c of candidates){
  const rs=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===c.profile);
  const failures=rs.filter(r=>r.type==='tetrp_reset_screen_failure');
  const games=rs.filter(r=>r.type==='tetrp_synchronous_match');
  const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));
  for(const g of games){
    const k=g.screen_pair?.pair_seeds?.join('-');
    if(!expectedKeys.has(k))throw new Error('unexpected pair '+k);
    if(g.node_budget_per_decision!==200000||g.frames_per_piece!==24||g.tetrp_ref!==tetrpRef)throw new Error('protocol mismatch');
    if(g.scored!==true||g.diagnostic_stop_reason!==null)throw new Error('unscored game entered confirmation');
    byPair.get(k).push(g);
  }
  let cs=0,is=0,sp=0; const problems=[];
  for(const p of expectedPairs){
    const a=byPair.get(p.join('-'));
    const slots=a.map(x=>x.screen_pair.candidate_slot).sort();
    if(a.length!==2||slots[0]!==0||slots[1]!==1){problems.push({pair:p,count:a.length,slots});continue;}
    const w=a.filter(x=>x.winner_profile===c.profile).length;
    if(w===2)cs++; else if(w===0)is++; else sp++;
  }
  const cw=games.filter(g=>g.winner_profile===c.profile).length;
  const iw=games.filter(g=>g.winner_profile===incumbent).length;
  const complete=failures.length===0&&games.length===32&&problems.length===0;
  let classification='incomplete_unscored';
  if(complete){
    classification=cw>=20?'confirmed_candidate':cw<=12?'rejected_for_now':'confirmation_inconclusive';
  }

  let cp=0,cg=0,cc=0,csent=0,ip=0,ig=0,ic=0,isnt=0;
  for(const g of games){
    const a=g.slots.find(s=>s.profile===c.profile), b=g.slots.find(s=>s.profile===incumbent);
    if(!a||!b)throw new Error('profile summary mismatch');
    cp+=a.pieces;cg+=a.generated;cc+=a.cancelled;csent+=a.sent;
    ip+=b.pieces;ig+=b.generated;ic+=b.cancelled;isnt+=b.sent;
  }
  const locks=games.map(g=>g.lock_steps);
  summaries.push({
    id:c.id,profile:c.profile,complete,classification,
    ko:{candidate_wins:cw,incumbent_wins:iw,candidate_sweeps:cs,incumbent_sweeps:is,split_pairs:sp,paired_sign_p:signP(cs,is)},
    lock_steps:{average:mean(locks),median:median(locks),min:locks.length?Math.min(...locks):null,max:locks.length?Math.max(...locks):null},
    diagnostics:{
      candidate:{pieces:cp,generated:cg,cancelled:cc,sent:csent,raw_app:cp?cg/cp:null,sent_app:cp?csent/cp:null},
      incumbent:{pieces:ip,generated:ig,cancelled:ic,sent:isnt,raw_app:ip?ig/ip:null,sent_app:ip?isnt/ip:null},
    },
    failures:failures.map(f=>({pair_seeds:f.pair_seeds,error:f.error})),
    pair_problems:problems,
  });
}

const allComplete=summaries.every(x=>x.complete);
const out={
  experiment:'Tetrp reset standalone confirmation',
  protocol:{
    incumbent_profile:incumbent,node_budget_per_decision:200000,frames_per_piece:24,pps:2.5,
    paired_units_per_candidate:16,games_per_candidate:32,authority_seed_pairs:expectedPairs,
    decision_rule:{confirmed_candidate:'>=20/32',rejected_for_now:'<=12/32',confirmation_inconclusive:'13-19/32'},
    guard:'KO only. Exact paired sign p and APP are diagnostics and do not override the preregistered KO rule.'
  },
  all_complete:allComplete,candidates:summaries
};
fs.mkdirSync(path.dirname(outputPath),{recursive:true});
fs.writeFileSync(outputPath,JSON.stringify(out,null,2)+'\n');
process.stdout.write(JSON.stringify(out,null,2)+'\n');
if(!allComplete)process.exitCode=2;
