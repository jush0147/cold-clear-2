import fs from 'node:fs';
import path from 'node:path';

export function canonicalVector(vector, parameterNames) {
  const out={};
  for(const name of parameterNames) out[name]=Number(vector[name]);
  return out;
}

export function profileForVector(vector, parameterNames) {
  return 'tuner:'+JSON.stringify(canonicalVector(vector, parameterNames));
}

function hash32(x) {
  x|=0;
  x=Math.imul(x^(x>>>16),0x45d9f3b);
  x=Math.imul(x^(x>>>16),0x45d9f3b);
  return (x^(x>>>16))>>>0;
}

function rngFor(seed,generation) {
  let state=hash32((seed>>>0)^Math.imul(generation+1,0x9e3779b1));
  return ()=>{
    state=(Math.imul(1664525,state)+1013904223)>>>0;
    return (state+0.5)/4294967296;
  };
}

function gaussian(rng) {
  const u1=Math.max(Number.EPSILON,rng());
  const u2=rng();
  return Math.sqrt(-2*Math.log(u1))*Math.cos(2*Math.PI*u2);
}

function reflect(value,min,max) {
  if(!(max>min)) throw new Error('parameter max must exceed min');
  const span=max-min;
  let x=value;
  for(let i=0;i<16&&(x<min||x>max);i++) {
    if(x<min) x=min+(min-x);
    if(x>max) x=max-(x-max);
  }
  if(x<min||x>max) {
    const wrapped=((x-min)%(2*span)+2*span)%(2*span);
    x=wrapped<=span?min+wrapped:max-(wrapped-span);
  }
  return Math.min(max,Math.max(min,x));
}

function round(value,places=8) {
  const p=10**places;
  return Math.round(value*p)/p;
}

export function initialState(config) {
  const mean={},sigma={};
  for(const [name,p] of Object.entries(config.parameters)) {
    mean[name]=p.initial_mean;
    sigma[name]=p.initial_sigma;
  }
  return {
    schema:'tetrp-tuner-state/1',
    algorithm:'diagonal_es_v1',
    generation:0,
    mean,
    sigma,
    last_generation:null,
  };
}

export function askGeneration(config,stateInput=null) {
  const state=stateInput??initialState(config);
  if(state.schema!=='tetrp-tuner-state/1') throw new Error('unsupported tuner state schema');
  if(config.algorithm!=='diagonal_es_v1'||state.algorithm!=='diagonal_es_v1') {
    throw new Error('unsupported tuner algorithm');
  }
  const names=Object.keys(config.parameters);
  const population=config.population;
  if(!Number.isInteger(population)||population<4||population%2!==0) {
    throw new Error('population must be an even integer >= 4');
  }
  const rng=rngFor(config.random_seed,state.generation);
  const z=[];
  for(let i=0;i<population/2;i++) {
    const row={};
    for(const name of names) row[name]=gaussian(rng);
    z.push(row);
  }
  const candidates=[];
  for(let i=0;i<population;i++) {
    const source=z[i%(population/2)];
    const sign=i<population/2?1:-1;
    const vector={};
    for(const name of names) {
      const p=config.parameters[name];
      const raw=state.mean[name]+sign*state.sigma[name]*source[name];
      vector[name]=round(reflect(raw,p.min,p.max));
    }
    const candidate_id='g'+String(state.generation).padStart(2,'0')+'-c'+String(i).padStart(2,'0');
    candidates.push({
      candidate_id,
      generation:state.generation,
      vector,
      vector_json:JSON.stringify(canonicalVector(vector,names)),
      profile:profileForVector(vector,names),
      pair_seed_base:config.pair_seed_base+state.generation*config.pair_seed_stride,
      pair_count:config.pairs_per_candidate,
    });
  }
  return {
    schema:'tetrp-tuner-ask/1',
    evidence_role:config.evidence_role,
    generation:state.generation,
    algorithm:state.algorithm,
    parameter_names:names,
    state,
    candidates,
    matrix:{include:candidates},
  };
}

function walk(dir) {
  return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>{
    const p=path.join(dir,e.name);
    return e.isDirectory()?walk(p):[p];
  });
}

export function loadRows(dir) {
  const rows=[];
  for(const file of walk(dir)) {
    if(!file.endsWith('.jsonl')) continue;
    for(const [i,line] of fs.readFileSync(file,'utf8').split(/\r?\n/).entries()) {
      if(!line.trim()) continue;
      let row;
      try { row=JSON.parse(line); }
      catch(error) { throw new Error('invalid JSON '+file+':'+(i+1)+': '+error.message); }
      if(row.type==='tetrp_synchronous_match'||row.type==='tetrp_reset_screen_failure') rows.push(row);
    }
  }
  return rows;
}

function summarizeCandidate(candidate,rows) {
  const matched=rows.filter(r=>(r.screen_pair?.candidate_profile??r.candidate_profile)===candidate.profile);
  const failures=matched.filter(r=>r.type==='tetrp_reset_screen_failure');
  const games=matched.filter(r=>r.type==='tetrp_synchronous_match');
  const expectedPairs=Array.from({length:candidate.pair_count},(_,i)=>[
    candidate.pair_seed_base+i*2,
    candidate.pair_seed_base+i*2+1,
  ]);
  const byPair=new Map(expectedPairs.map(p=>[p.join('-'),[]]));
  for(const game of games) {
    const key=game.screen_pair?.pair_seeds?.join('-');
    if(!byPair.has(key)) throw new Error('unexpected pair '+key+' for '+candidate.candidate_id);
    byPair.get(key).push(game);
  }
  let candidateSweeps=0,incumbentSweeps=0,splitPairs=0;
  const pairProblems=[];
  for(const pair of expectedPairs) {
    const gs=byPair.get(pair.join('-'));
    const slots=gs.map(g=>g.screen_pair.candidate_slot).sort();
    if(gs.length!==2||slots[0]!==0||slots[1]!==1) {
      pairProblems.push({pair,count:gs.length,candidate_slots:slots});
      continue;
    }
    const wins=gs.filter(g=>g.winner_profile===candidate.profile).length;
    if(wins===2) candidateSweeps++;
    else if(wins===0) incumbentSweeps++;
    else splitPairs++;
  }
  const candidateWins=games.filter(g=>g.winner_profile===candidate.profile).length;
  const complete=failures.length===0&&pairProblems.length===0&&games.length===candidate.pair_count*2;
  return {
    candidate_id:candidate.candidate_id,
    profile:candidate.profile,
    vector:candidate.vector,
    complete,
    ko:{candidate_wins:candidateWins,incumbent_wins:games.length-candidateWins},
    pairs:{
      candidate_sweeps:candidateSweeps,
      incumbent_sweeps:incumbentSweeps,
      split_pairs:splitPairs,
      net_sweeps:candidateSweeps-incumbentSweeps,
    },
    failures:failures.map(f=>({pair_seeds:f.pair_seeds,error:f.error})),
    pair_problems:pairProblems,
  };
}

function rankResults(a,b) {
  return b.pairs.net_sweeps-a.pairs.net_sweeps ||
    b.ko.candidate_wins-a.ko.candidate_wins ||
    a.candidate_id.localeCompare(b.candidate_id);
}

export function tellGeneration(config,ask,rows) {
  if(ask.schema!=='tetrp-tuner-ask/1') throw new Error('unsupported ask schema');
  const results=ask.candidates.map(c=>summarizeCandidate(c,rows));
  if(results.some(r=>!r.complete)) {
    return {
      schema:'tetrp-tuner-tell/1',
      evidence_role:config.evidence_role,
      generation:ask.generation,
      complete:false,
      results,
      next_state:null,
    };
  }
  const ranked=[...results].sort(rankResults);
  const mu=Math.max(1,Math.floor(config.population*config.elite_fraction));
  const elite=ranked.slice(0,mu);
  const rawWeights=elite.map((_,i)=>Math.log(mu+0.5)-Math.log(i+1));
  const weightSum=rawWeights.reduce((a,b)=>a+b,0);
  const weights=rawWeights.map(w=>w/weightSum);
  const nextMean={},nextSigma={};
  for(const [name,p] of Object.entries(config.parameters)) {
    nextMean[name]=round(elite.reduce((sum,r,i)=>sum+weights[i]*r.vector[name],0));
    const spread=Math.sqrt(elite.reduce((sum,r,i)=>
      sum+weights[i]*(r.vector[name]-nextMean[name])**2,0));
    const old=ask.state.sigma[name];
    const adapted=0.65*old+0.35*Math.max(spread,p.min_sigma);
    nextSigma[name]=round(Math.min(p.max_sigma,Math.max(p.min_sigma,adapted)));
  }
  const nextState={
    schema:'tetrp-tuner-state/1',
    algorithm:'diagonal_es_v1',
    generation:ask.generation+1,
    mean:nextMean,
    sigma:nextSigma,
    last_generation:{
      generation:ask.generation,
      best_candidate_id:ranked[0].candidate_id,
      best_vector:ranked[0].vector,
      best_net_sweeps:ranked[0].pairs.net_sweeps,
      best_ko_wins:ranked[0].ko.candidate_wins,
    },
  };
  return {
    schema:'tetrp-tuner-tell/1',
    evidence_role:config.evidence_role,
    generation:ask.generation,
    complete:true,
    ranking:ranked.map(r=>r.candidate_id),
    elite:elite.map(r=>r.candidate_id),
    results,
    next_state:nextState,
  };
}

export function readJson(file) {
  return JSON.parse(fs.readFileSync(file,'utf8'));
}
