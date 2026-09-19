import fs from 'node:fs';
import { askGeneration, readJson } from './controller.mjs';

const [configPath,statePath='-',outputPath='tuner-ask.json']=process.argv.slice(2);
if(!configPath) throw new Error('usage: node ask.mjs <config.json> [state.json|-] [output.json]');
const config=readJson(configPath);
const state=statePath==='-'?null:readJson(statePath);
const ask=askGeneration(config,state);
fs.writeFileSync(outputPath,JSON.stringify(ask,null,2)+'\n');
if(process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(process.env.GITHUB_OUTPUT,'matrix='+JSON.stringify(ask.matrix)+'\n');
  fs.appendFileSync(process.env.GITHUB_OUTPUT,'generation='+ask.generation+'\n');
}
process.stdout.write(JSON.stringify({
  generation:ask.generation,
  evidence_role:ask.evidence_role,
  candidates:ask.candidates.map(c=>({id:c.candidate_id,vector:c.vector})),
})+'\n');
