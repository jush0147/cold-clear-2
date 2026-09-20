import fs from 'node:fs';
import { loadRows, readJson, tellGeneration } from './controller.mjs';

const [configPath,askPath,resultsDir,stateOut='tuner-next-state.json',summaryOut='tuner-summary.json']=process.argv.slice(2);
if(!configPath||!askPath||!resultsDir) {
  throw new Error('usage: node tell.mjs <config.json> <ask.json> <results-dir> [state-out] [summary-out]');
}
const config=readJson(configPath);
const ask=readJson(askPath);
const rows=loadRows(resultsDir);
const summary=tellGeneration(config,ask,rows);
fs.writeFileSync(summaryOut,JSON.stringify(summary,null,2)+'\n');
if(!summary.complete) {
  process.stdout.write(JSON.stringify(summary,null,2)+'\n');
  process.exitCode=2;
} else {
  fs.writeFileSync(stateOut,JSON.stringify(summary.next_state,null,2)+'\n');
  if(process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT,'generation='+summary.generation+'\n');
    fs.appendFileSync(process.env.GITHUB_OUTPUT,'best_candidate='+summary.ranking[0]+'\n');
  }
  process.stdout.write(JSON.stringify({
    generation:summary.generation,
    best_candidate:summary.ranking[0],
    next_state:summary.next_state,
  })+'\n');
}
