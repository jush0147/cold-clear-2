import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const root=path.resolve(process.argv[2]||'kiwi-v1-browser');
function files(dir) {
  return fs.readdirSync(dir,{withFileTypes:true}).flatMap(entry=>{
    const file=path.join(dir,entry.name);
    if(entry.isDirectory())return files(file);
    if(!entry.isFile())throw new Error('Unexpected non-regular artifact entry');
    return [path.relative(root,file).split(path.sep).join('/')];
  });
}
const hashes=JSON.parse(fs.readFileSync(path.join(root,'sha256.json'),'utf8'));
const actual=files(root).filter(name=>name!=='sha256.json').sort();
assert.deepEqual(actual,Object.keys(hashes).sort(),'Downloaded artifact file set must match hash manifest exactly');
for(const name of actual) {
  assert.ok(!name.split('/').some(part=>part.startsWith('.')),'Artifact must contain no implicit hidden files');
  const digest=crypto.createHash('sha256').update(fs.readFileSync(path.join(root,name))).digest('hex');
  assert.equal(digest,hashes[name],'SHA-256 mismatch: '+name);
}
// Hash manifests prove identity, not module closure. Verify every static relative
// ESM import resolves inside the delivered package.
for(const name of actual.filter(n=>n.endsWith('.js')||n.endsWith('.mjs'))) {
  const source=fs.readFileSync(path.join(root,name),'utf8');
  const re=/(?:from\s*|import\s*)['"]((?:\.\.\/|\.\/)[^'"]+)['"]/g;
  for(const match of source.matchAll(re)) {
    const target=path.normalize(path.join(path.dirname(name),match[1])).split(path.sep).join('/');
    assert.ok(actual.includes(target),'Missing packaged relative import: '+name+' -> '+match[1]);
  }
}
const read=name=>JSON.parse(fs.readFileSync(path.join(root,name),'utf8'));
const build=read('kiwi-build.json');
assert.equal(build.schema,'tetrp-kiwi-build/3');
assert.equal(build.product_version,'kiwi-v1-snapshot-v3.2');
assert.equal(build.snapshot_api,'analyze_snapshot_json');
assert.equal(build.request_schema,'kiwi-snapshot/3');
assert.equal(build.result_schema,'kiwi-snapshot-result/3');
assert.equal(build.capabilities.same_piece_hold_search,true);
assert.equal(build.capabilities.hold_information_gain_optimized,false);
assert.equal(build.capabilities.root_geometry_in_search,true);
assert.equal(build.capabilities.rules_parity_verified,false);
assert.equal(build.capabilities.pending_unknown_activation,'three_explicit_timing_scenarios_shared_node_budget');
assert.equal(read('kiwi-snapshot-acceptance.json').status,'passed');
assert.equal(read('kiwi-snapshot-browser.json').status,'passed');
assert.equal(read('kiwi-package-e2e.json').status,'passed');
assert.equal(read('kiwi-rule-fixture-counts.json').schema,'kiwi-rule-fixtures/2');
console.log(JSON.stringify({status:'passed',verified_files:actual.length,exact_file_set:true,source_commit:read('kiwi-build.json').source_commit},null,2));
