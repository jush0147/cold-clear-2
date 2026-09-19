import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { runSynchronousMatch } from './lib/tetrp-synchronous-match.mjs';

const require=createRequire(import.meta.url);
const { analyze_pending_profile_json }=require('../pkg-node/cold_clear_2.js');

const tetrpRoot=path.resolve(process.argv[2]||'tetrp-reference');
const {Engine}=await import(pathToFileURL(path.join(tetrpRoot,'src/engine.js')).href);
const B=await import(pathToFileURL(path.join(tetrpRoot,'src/board.js')).href);
const R=await import(pathToFileURL(path.join(tetrpRoot,'src/rotation.js')).href);

const seed=Number(process.env.MATCH_SEED||61000);
const nodeBudget=Number(process.env.NODE_BUDGET||200000);
const framesPerPiece=Number(process.env.FRAMES_PER_PIECE||30);
const profiles=[
  process.env.SLOT0_PROFILE||'review_h9_h12',
  process.env.SLOT1_PROFILE||'corrected_legacy_h12',
];
const tracePath=process.env.TRACE_OUT||null;
const progressEveryLockSteps=Number(process.env.PROGRESS_EVERY_LOCKS||250);
const diagnosticMaxLockSteps=process.env.DIAGNOSTIC_MAX_LOCKS
  ? Number(process.env.DIAGNOSTIC_MAX_LOCKS)
  : null;
const stopOnFirstDecisionDivergence=process.env.STOP_ON_FIRST_DECISION_DIVERGENCE==='1';

const out=runSynchronousMatch({
  Engine,
  boardModule:B,
  rotationModule:R,
  analyzeProfileJson:analyze_pending_profile_json,
  seed,
  nodeBudget,
  framesPerPiece,
  profiles,
  tetrpRef:process.env.TETRP_REF||null,
  trace:Boolean(tracePath),
  safetyLockSteps:Number(process.env.SAFETY_LOCK_STEPS||5000),
  progressEveryLockSteps,
  onProgress:progressEveryLockSteps>0
    ? (progress)=>process.stderr.write(JSON.stringify(progress)+'\n')
    : null,
  diagnosticMaxLockSteps,
  stopOnFirstDecisionDivergence,
});

if(tracePath) writeFileSync(tracePath,JSON.stringify(out.trace,null,2)+'\n');
process.stdout.write(JSON.stringify(out.result)+'\n');
