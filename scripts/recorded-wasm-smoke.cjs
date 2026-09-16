// Offline recorded-game fixture regression. No network or account access.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const {analyze_pending_json, check_replay_lock_json} = require(path.join(root, 'pkg-node/cold_clear_2.js'));
const fixture = JSON.parse(fs.readFileSync(path.join(root, 'tests/fixtures/replay-v19.json'), 'utf8'));
for (const c of fixture.cases) {
  const start = {...c.start, board: c.rows.map(r => [...r].map(v => v === '.' ? null : v))};
  const o = JSON.parse(check_replay_lock_json(JSON.stringify({start, placement: c.placement})));
  assert.deepEqual({lines: o.lines, combo: o.consecutive_clears,
    b2b: o.back_to_back ? o.b2b_count + 1 : 0,
    garbage_cleared: o.garbage_cleared, raw_attack: o.packets,
    surge: o.attack.surge_released, pc: o.perfect_clear}, c.expected, `fixture ${c.index}`);
  if (c.after !== null) {
    const rows = o.board.map(row => row.map(v => v ? 'X' : '.').join(''));
    while (rows.at(-1) === '..........') rows.pop();
    assert.deepEqual(rows, c.after, `board ${c.index}`);
  }
}
const start = {board: [], queue: ['I', 'O', 'T', 'L', 'J', 'S'], hold: 'Z',
  combo: 0, back_to_back: false, b2b_count: 0, randomizer: {type: 'unknown'}};
const request = {start, incoming: [{lines: 8, active: true}], pieces_placed: 30,
  garbage_sent: 0, activation_model: "snapshot", iterations: 10};
const danger = JSON.parse(analyze_pending_json(JSON.stringify(request)));
const empty = JSON.parse(analyze_pending_json(JSON.stringify({...request, incoming: []})));
assert(danger.candidates.length > 0 && empty.candidates.length > 0);
assert.equal(danger.pending_garbage_in_search, true);
assert(danger.candidates[0].mean_score < empty.candidates[0].mean_score);
assert.throws(() => analyze_pending_json(JSON.stringify({...request,
  start: {...start, queue: [...start.queue, 'Z']}})));
assert.throws(() => analyze_pending_json(JSON.stringify({...request, seed: 123})));
console.log(JSON.stringify({runtime: 'Node WASM', status: 'passed',
  recorded_fixtures: fixture.cases.length, pending_search_changes_scores: true,
  hidden_input_rejected: true, pending_nodes: danger.nodes,
  full_rules_parity_verified: false}, null, 2));
