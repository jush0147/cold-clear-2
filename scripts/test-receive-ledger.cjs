'use strict';
const assert=require('node:assert/strict');
const {ReceiveLedger}=require('./receive-ledger.cjs');
const l=new ReceiveLedger();
l.apply(['r',0,8]);l.apply(['c',0,3]);l.apply(['f',0]);l.apply(['f',0]);
assert.equal(l.confirmed,5);l.apply(['c',0,2]);l.apply(['t',0,3]);assert.equal(l.summary().confirmed,5);
l.apply(['r',1,4]);l.apply(['c',1,4]);l.apply(['f',1]);assert.equal(l.confirmed,5);
l.apply(['r',2,7]);assert.equal(l.summary().unconfirmed_queued,7);
assert.throws(()=>l.apply(['t',2,1]));assert.throws(()=>l.apply(['r',2,4]));
// Twenty independently recorded endpoints; columns are admitted, pre-confirm
// cancellations, still-unconfirmed at end, and official received. This checks
// the accounting identity, not a re-execution of all original replay events.
const rows=[[20,0,0,20],[93,0,0,93],[141,9,0,132],[122,10,6,106],
[122,4,0,118],[135,3,0,132],[60,5,0,55],[106,4,5,97],[36,5,0,31],[8,0,0,8],
[31,1,0,30],[68,1,6,61],[153,10,0,143],[183,16,0,167],[4,0,0,4],[25,0,0,25],
[57,1,0,56],[28,5,0,23],[171,11,0,160],[158,1,0,157]];
for(const [admitted,cancelled,pending,official] of rows)assert.equal(admitted-cancelled-pending,official);
// Optional complete local replay traces. Raw player recordings are not committed.
if(process.argv[2]){
 const trace=JSON.parse(require('node:fs').readFileSync(process.argv[2],'utf8'));
 for(const s of trace.streams){const t=new ReceiveLedger();for(const e of s.events)t.apply(e);assert.equal(t.summary().confirmed,s.expected_received);}
}
console.log(JSON.stringify({status:'passed',recorded_accounting_summaries:rows.length,
  complete_trace_reexecution:!!process.argv[2],duplicate_and_late_confirmation_tests:true},null,2));
