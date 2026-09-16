'use strict';
// Statistical audit only. Never changes the engine's board, queue, or RNG.
const assert = require('node:assert/strict');
class ReceiveLedger {
  constructor() {
    this.packets = new Map(); this.admitted = 0; this.confirmed = 0;
    this.cancelledBeforeConfirm = 0; this.cancelledAfterConfirm = 0; this.tanked = 0;
  }
  apply([op, id, lines = 0]) {
    assert(Number.isSafeInteger(id) && id >= 0, 'invalid packet id');
    const p = this.packets.get(id);
    if (op === 'r') {
      assert(Number.isSafeInteger(lines) && lines > 0, 'invalid admitted amount');
      assert(!p, 'duplicate packet admission');
      this.packets.set(id, {remaining: lines, confirmed: false}); this.admitted += lines;
    } else if (op === 'f') {
      if (p && !p.confirmed) {this.confirmed += p.remaining; p.confirmed = true;}
    } else if (op === 'c' || op === 't') {
      assert(p && Number.isSafeInteger(lines) && lines > 0 && lines <= p.remaining, 'invalid packet removal');
      if (op === 't') {assert(p.confirmed, 'unconfirmed packet cannot be tanked'); this.tanked += lines;}
      else if (p.confirmed) this.cancelledAfterConfirm += lines;
      else this.cancelledBeforeConfirm += lines;
      p.remaining -= lines;
    } else throw new Error(`unknown ledger operation: ${op}`);
  }
  summary() {
    let queued = 0, unconfirmedQueued = 0;
    for (const p of this.packets.values()) {queued += p.remaining; if (!p.confirmed) unconfirmedQueued += p.remaining;}
    assert.equal(this.admitted, this.cancelledBeforeConfirm + this.cancelledAfterConfirm + this.tanked + queued);
    assert.equal(this.confirmed, this.admitted - this.cancelledBeforeConfirm - unconfirmedQueued);
    return {admitted: this.admitted, confirmed: this.confirmed,
      cancelled_before_confirmation: this.cancelledBeforeConfirm,
      cancelled_after_confirmation: this.cancelledAfterConfirm,
      tanked: this.tanked, queued, unconfirmed_queued: unconfirmedQueued};
  }
}
function attachReceiveLedger(engine) {
  const ledger = new ReceiveLedger(), events = [], ids = new Map();
  const idFor = (cid, gameid) => {
    const key = JSON.stringify([gameid,cid]);
    if (!ids.has(key)) ids.set(key,ids.size);
    return ids.get(key);
  };
  const record = event => {ledger.apply(event); events.push(event);};
  const q = engine.garbageQueue;
  const receive = q.receive.bind(q);
  q.receive = (...args) => {
    receive(...args);
    for (const g of args) if (g.amount > 0) record(['r',idFor(g.cid,g.gameid),g.amount]);
    assert.equal(q.size,ledger.summary().queued, 'unsupported queue cap or receipt semantics');
  };
  const confirm = q.confirm.bind(q);
  q.confirm = (cid,gameid,frame) => {const result = confirm(cid,gameid,frame); record(['f',idFor(cid,gameid)]); return result;};
  const cancel = q.cancel.bind(q);
  q.cancel = (...args) => {
    const result = cancel(...args);
    for (const g of result[1]) record(['c',idFor(g.cid,g.gameid),g.amount]);
    assert.equal(q.size,ledger.summary().queued); return result;
  };
  const tank = q.tank.bind(q);
  q.tank = (...args) => {
    const result = tank(...args), removed = new Map();
    for (const g of result) {const id = idFor(g.id,g.gameid); removed.set(id,(removed.get(id)||0)+g.amount);}
    for (const [id,n] of removed) record(['t',id,n]);
    assert.equal(q.size,ledger.summary().queued); return result;
  };
  return {ledger,events};
}
module.exports = {ReceiveLedger,attachReceiveLedger};
