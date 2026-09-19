import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const dir=path.resolve(process.argv[2]||'rounds');
const out=path.resolve(process.argv[3]||'exhibition/tuned-vs-legacy-ft7-parallel.ttrm');
const files=readdirSync(dir).filter(f=>/^round-\d+\.ttrm$/.test(f)).sort((a,b)=>{
  const sa=Number(a.match(/\d+/)[0]), sb=Number(b.match(/\d+/)[0]); return sa-sb;
});
if(files.length<13) throw new Error('need 13 generated seed rounds, found '+files.length);

let template=null,tuned=0,legacy=0;
const rounds=[],summary=[];
for(const file of files){
  const doc=JSON.parse(readFileSync(path.join(dir,file),'utf8'));
  if(!template) template=doc;
  const seed=doc.synthetic.seed;
  const winner=doc.synthetic.winner;
  const round=doc.replay?.rounds?.[0];
  if(!Array.isArray(round)||round.length!==2) throw new Error('invalid round '+file);
  if(winner==='TUNED H9') tuned++;
  else if(winner==='CORRECTED LEGACY') legacy++;
  else throw new Error('non-unique winner at seed '+seed+': '+winner);
  rounds.push(round);
  summary.push({round:rounds.length,seed,winner,score_after:{tuned,legacy},final:doc.synthetic.final});
  if(tuned===7||legacy===7) break;
}
if(tuned!==7&&legacy!==7) throw new Error('13 decisive rounds did not resolve FT7');
const winner=tuned===7?'TUNED H9':'CORRECTED LEGACY';
const output={
  ...template,
  ts:new Date().toISOString(),
  replay:{rounds},
  synthetic:{
    purpose:'visualize a complete fixed-2-PPS cold-clear-2 FT7 without modifying Tetrp',
    format:'FT7',
    seed_start:template.synthetic.seed,
    node_budget:template.synthetic.node_budget,
    frames_per_piece:template.synthetic.frames_per_piece,
    profiles:template.synthetic.profiles,
    names:template.synthetic.names,
    authority:'jush0147/tetrp engine',
    warning:'Synthetic exhibition only. Do not pool this match into strategy-strength evidence.',
    winner,tuned_wins:tuned,legacy_wins:legacy,rounds:summary,
  }
};
writeFileSync(out,JSON.stringify(output,null,2)+'\n');
writeFileSync(out+'.summary.json',JSON.stringify(output.synthetic,null,2)+'\n');
console.log(JSON.stringify({type:'ft7_summary',rounds:rounds.length,tuned_wins:tuned,legacy_wins:legacy,winner,output:out},null,2));
