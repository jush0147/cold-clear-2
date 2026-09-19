import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { runSynchronousMatch } from './lib/tetrp-synchronous-match.mjs';

const require=createRequire(import.meta.url);
const { analyze_pending_profile_json }=require('../pkg-node/cold_clear_2.js');

const tetrpRoot=path.resolve(process.argv[2]||'tetrp-reference');
const {Engine}=await import(pathToFileURL(path.join(tetrpRoot,'src/engine.js')).href);
const B=await import(pathToFileURL(path.join(tetrpRoot,'src/board.js')).href);
const R=await import(pathToFileURL(path.join(tetrpRoot,'src/rotation.js')).href);

const seedA=Number(process.env.SEED_A);
const seedB=Number(process.env.SEED_B);
const candidateProfile=process.env.CANDIDATE_PROFILE;
const incumbentProfile=process.env.INCUMBENT_PROFILE||'corrected_legacy_h12';
const nodeBudget=Number(process.env.NODE_BUDGET||200000);
const framesPerPiece=Number(process.env.FRAMES_PER_PIECE||24);
const safetyLockSteps=Number(process.env.SAFETY_LOCK_STEPS||5000);
const progressEveryLockSteps=Number(process.env.PROGRESS_EVERY_LOCKS||250);
const tetrpRef=process.env.TETRP_REF||null;

if(!Number.isSafeInteger(seedA)||!Number.isSafeInteger(seedB)||seedA===seedB) {
  throw new Error('SEED_A and SEED_B must be distinct safe integers');
}
if(!candidateProfile) throw new Error('CANDIDATE_PROFILE is required');
if(candidateProfile===incumbentProfile) throw new Error('candidate and incumbent profiles must differ');

const games=[
  {
    game_in_pair:0,
    candidate_slot:0,
    profiles:[candidateProfile,incumbentProfile],
    candidate_seed:seedA,
    incumbent_seed:seedB,
  },
  {
    game_in_pair:1,
    candidate_slot:1,
    profiles:[incumbentProfile,candidateProfile],
    candidate_seed:seedB,
    incumbent_seed:seedA,
  },
];

for(const spec of games) {
  const meta={
    pair_seeds:[seedA,seedB],
    game_in_pair:spec.game_in_pair,
    candidate_profile:candidateProfile,
    incumbent_profile:incumbentProfile,
    candidate_slot:spec.candidate_slot,
    candidate_seed:spec.candidate_seed,
    incumbent_seed:spec.incumbent_seed,
  };
  try {
    const out=runSynchronousMatch({
      Engine,
      boardModule:B,
      rotationModule:R,
      analyzeProfileJson:analyze_pending_profile_json,
      seeds:[seedA,seedB],
      nodeBudget,
      framesPerPiece,
      profiles:spec.profiles,
      tetrpRef,
      trace:false,
      safetyLockSteps,
      progressEveryLockSteps,
      onProgress:progressEveryLockSteps>0
        ? (progress)=>process.stderr.write(JSON.stringify({...progress,screen_pair:meta})+'\n')
        : null,
    });
    process.stdout.write(JSON.stringify({...out.result,screen_pair:meta})+'\n');
  } catch(error) {
    process.stdout.write(JSON.stringify({
      type:'tetrp_reset_screen_failure',
      ...meta,
      node_budget_per_decision:nodeBudget,
      frames_per_piece:framesPerPiece,
      tetrp_ref:tetrpRef,
      error:error instanceof Error?error.message:String(error),
    })+'\n');
  }
}
