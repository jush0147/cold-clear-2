import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const tetrpRoot = path.resolve(process.argv[2] || 'tetrp-reference');
const outPath = path.resolve(process.argv[3] || 'bot-exhibition-ft7.ttrm');
const seedStart = Number(process.env.EXHIBITION_SEED || 49000);
const nodeBudget = Number(process.env.NODE_BUDGET || 200000);
const targetWins = Number(process.env.TARGET_WINS || 7);
if (!Number.isSafeInteger(seedStart) || !Number.isSafeInteger(nodeBudget) || nodeBudget < 1000 ||
    !Number.isSafeInteger(targetWins) || targetWins < 1 || targetWins > 20) {
  throw new Error('invalid FT target/seed/node budget');
}

const generator = path.resolve('scripts/generate-tetrp-exhibition.mjs');
const tmp = mkdtempSync(path.join(os.tmpdir(), 'tetrp-ft7-'));
const rounds = [];
const roundSummaries = [];
let tunedWins = 0, legacyWins = 0, roundIndex = 0;
let template = null;

try {
  while (tunedWins < targetWins && legacyWins < targetWins) {
    if (roundIndex >= 39) throw new Error('FT set exceeded 39-round safety cap');
    const seed = seedStart + roundIndex;
    const single = path.join(tmp, 'round-'+String(roundIndex).padStart(2,'0')+'.ttrm');
    const stdout = execFileSync(process.execPath, [generator, tetrpRoot, single], {
      env: { ...process.env, EXHIBITION_SEED: String(seed), NODE_BUDGET: String(nodeBudget) },
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
    });
    process.stdout.write(stdout);

    const doc = JSON.parse(readFileSync(single, 'utf8'));
    if (!template) template = doc;
    const round = doc.replay?.rounds?.[0];
    if (!Array.isArray(round) || round.length !== 2) throw new Error('single-round generator emitted invalid league round');
    rounds.push(round);

    const winner = doc.synthetic?.winner;
    if (winner === 'TUNED H9') tunedWins++;
    else if (winner === 'CORRECTED LEGACY') legacyWins++;
    else throw new Error('single round had no unique winner: '+String(winner));

    roundSummaries.push({
      round: roundIndex + 1,
      seed,
      winner,
      score_after: { tuned: tunedWins, legacy: legacyWins },
      final: doc.synthetic.final,
    });
    console.log(JSON.stringify({
      type:'ft7_round',
      round:roundIndex+1,
      seed,
      winner,
      tuned_wins:tunedWins,
      legacy_wins:legacyWins,
      final:doc.synthetic.final,
    }));
    roundIndex++;
  }

  const winner = tunedWins === targetWins ? 'TUNED H9' : 'CORRECTED LEGACY';
  const file = {
    ...template,
    ts: new Date().toISOString(),
    replay: { rounds },
    synthetic: {
      purpose:'visualize a complete fixed-2-PPS cold-clear-2 FT set without modifying Tetrp',
      format:'FT'+targetWins,
      seed_start:seedStart,
      node_budget:nodeBudget,
      frames_per_piece:template.synthetic.frames_per_piece,
      profiles:template.synthetic.profiles,
      names:template.synthetic.names,
      authority:'jush0147/tetrp engine',
      warning:'Synthetic exhibition only. Do not pool this match into strategy-strength evidence.',
      winner,
      tuned_wins:tunedWins,
      legacy_wins:legacyWins,
      rounds:roundSummaries,
    },
  };
  writeFileSync(outPath, JSON.stringify(file,null,2)+'\n');
  writeFileSync(outPath+'.summary.json', JSON.stringify(file.synthetic,null,2)+'\n');
  console.log(JSON.stringify({
    type:'ft7_summary',
    format:file.synthetic.format,
    rounds:rounds.length,
    tuned_wins:tunedWins,
    legacy_wins:legacyWins,
    winner,
    output:outPath,
  }, null, 2));
} finally {
  rmSync(tmp,{recursive:true,force:true});
}
