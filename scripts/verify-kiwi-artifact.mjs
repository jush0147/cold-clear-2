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
const read=name=>JSON.parse(fs.readFileSync(path.join(root,name),'utf8'));
assert.equal(read('kiwi-build.json').snapshot_api,'analyze_snapshot_json');
assert.equal(read('kiwi-snapshot-acceptance.json').status,'passed');
assert.equal(read('kiwi-snapshot-browser.json').status,'passed');
console.log(JSON.stringify({status:'passed',verified_files:actual.length,exact_file_set:true,source_commit:read('kiwi-build.json').source_commit},null,2));
