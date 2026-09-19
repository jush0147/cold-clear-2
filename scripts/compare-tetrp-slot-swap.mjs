import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const [abPath,baPath]=process.argv.slice(2);
if(!abPath||!baPath) throw new Error('usage: node compare-tetrp-slot-swap.mjs AB.diagnostic.json BA.diagnostic.json');
const ab=JSON.parse(readFileSync(abPath,'utf8'));
const ba=JSON.parse(readFileSync(baPath,'utf8'));

assert.equal(ab.diagnostics.length,ba.diagnostics.length,'slot swap changed trace length');

function compact(x) {
  return {
    profile:x.profile,
    pieces:x.pieces,
    app:x.app,
    placement:x.placement,
    path:x.path,
    pre_decision:x.pre_decision,
    bag:x.bag,
  };
}

for(let i=0;i<ab.diagnostics.length;i++) {
  const a=ab.diagnostics[i],b=ba.diagnostics[i];
  assert.deepEqual(compact(a.slot0),compact(b.slot1),'tuned trace changed after slot swap at piece '+(i+1));
  assert.deepEqual(compact(a.slot1),compact(b.slot0),'legacy trace changed after slot swap at piece '+(i+1));
}

const aFinal=ab.synthetic.final,bFinal=ba.synthetic.final;
assert.deepEqual(aFinal.tuned,bFinal.legacy,'slot0 AB final stats did not mirror slot1 BA');
assert.deepEqual(aFinal.legacy,bFinal.tuned,'slot1 AB final stats did not mirror slot0 BA');

console.log(JSON.stringify({
  ok:true,
  pieces:ab.diagnostics.length,
  checks:['profile trace mirrors across slot swap','placements and legal paths mirror','visible snapshots mirror','SevenBag observer mirrors']
},null,2));
