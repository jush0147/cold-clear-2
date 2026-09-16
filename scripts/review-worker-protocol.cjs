'use strict';
// Protocol-only test with a stub search. Real WASM is exercised separately.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const messages = [], tasks = new Map(), sessions = [];
let nextTimer = 0, resolveInit;
const initialized = new Promise(resolve => {resolveInit = resolve;});
class StubSession {
  constructor(input) {this.input = JSON.parse(input); this.steps = 0; this.freed = false; sessions.push(this);}
  step() {assert(!this.freed); return ++this.steps >= 2;}
  report_json() {return JSON.stringify({processed:this.steps,comparison:this.steps >= 2 ? {} : null});}
  candidate_json(id) {assert(!this.freed); return JSON.stringify({action_id:id});}
  free() {assert(!this.freed); this.freed = true;}
}
const source = fs.readFileSync(path.join(__dirname, '../web/review.worker.js'), 'utf8')
  .replace("import init, { ReviewSession } from '../pkg/cold_clear_2.js';", 'const init=__init, ReviewSession=__Session;');
const context = {self:{postMessage:m=>messages.push(m)},__init:()=>initialized,__Session:StubSession,
  setTimeout:fn=>{const id=++nextTimer;tasks.set(id,fn);return id;},clearTimeout:id=>tasks.delete(id)};
vm.runInNewContext(source,context);
const send=data=>context.self.onmessage({data});
function tick() {const [id,fn]=tasks.entries().next().value;tasks.delete(id);fn();}
(async()=>{
  const a=send({type:'review',id:'A',request:{}});
  const b=send({type:'review',id:'B',request:{}});
  resolveInit();await Promise.all([a,b]);assert.equal(sessions.length,1);
  await send({type:'cancel',id:'A'});assert.equal(tasks.size,1);
  tick();assert.equal(messages.at(-1).id,'B');assert.equal(messages.at(-1).type,'progress');
  await send({type:'review',id:'C',request:{}});assert(sessions[0].freed);
  tick();tick();assert.equal(messages.at(-1).type,'complete');assert.equal(messages.at(-1).id,'C');
  await send({type:'details',id:'C',action_id:4});assert.equal(messages.at(-1).details.action_id,4);
  await send({type:'cancel',id:'C'});assert(sessions[1].freed);assert.equal(tasks.size,0);
  assert(!messages.some(m=>m.type==='complete'&&m.id==='B'));
  console.log(JSON.stringify({status:'passed',scope:'stubbed worker protocol, not browser runtime',
    stale_init:true,stale_cancel:true,seek_disposes_previous:true,progress_and_details:true}));
})().catch(e=>{console.error(e);process.exitCode=1;});
