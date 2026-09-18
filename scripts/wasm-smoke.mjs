import assert from "node:assert/strict";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { WasmBot, analyze_pending_json, preview_garbage_one_to_one } = require("../pkg-node/cold_clear_2.js");

const initial = () => ({
  board: Array.from({ length: 20 }, () => Array(10).fill(null)),
  queue: ["O", "I", "T", "L", "J", "S"],
  hold: null,
  combo: 0,
  back_to_back: false,
  randomizer: { type: "unknown" },
});
const reviewInitial = () => ({
  ...initial(),
  randomizer: { type: "seven_bag", bag_state: ["Z"] },
});
const state = bot => JSON.parse(bot.player_state_json());
const o = JSON.stringify({ location: { type: "O", orientation: "north", x: 0, y: 0 }, spin: "none" });
const i = JSON.stringify({ location: { type: "I", orientation: "east", x: 0, y: 2 }, spin: "none" });

const bot = new WasmBot();
bot.start(JSON.stringify(initial()));
assert.deepEqual(state(bot).pieces, { current: "O", hold: null, next: ["I", "T", "L", "J", "S"] });
const nodes = bot.think(10);
const suggestion = JSON.parse(bot.suggest_json());
assert.equal(typeof nodes, "bigint");
assert.ok(nodes > 0n);
assert.ok(suggestion.length > 0);
assert.ok(JSON.parse(bot.stats_json()).expansions > 0);

const budgeted = new WasmBot();
budgeted.start(JSON.stringify(reviewInitial()));
assert.throws(() => budgeted.think_nodes(0));
const hardNodes = budgeted.think_nodes(1003);
assert.equal(hardNodes, 1003n);
const hardStats = JSON.parse(budgeted.stats_json());
assert.equal(hardStats.nodes, 1003);
assert.ok(hardStats.max_depth > 0);
budgeted.free();

const pendingReport = JSON.parse(analyze_pending_json(JSON.stringify({
  start: reviewInitial(),
  incoming: [{ lines: 8, active: true }],
  pieces_placed: 30,
  garbage_sent: 0,
  frames_per_piece: 12,
  pending_delay_frames: 20,
  node_budget: 1003,
})));
assert.equal(pendingReport.node_budget, 1003);
assert.ok(pendingReport.nodes <= 1003);
assert.equal(pendingReport.scenarios, 10);
assert.equal(pendingReport.config_profile, "h9+h12-review");
assert.ok(pendingReport.candidates.length > 0);

const before = bot.player_state_json();
assert.throws(() => bot.new_piece("Z"));
assert.throws(() => bot.play_json(JSON.stringify({ location: { type: "O", orientation: "north", x: 127, y: -128 }, spin: "none" })));
assert.equal(bot.player_state_json(), before);
assert.throws(() => bot.start(JSON.stringify({ ...initial(), queue: ["O", "I", "T", "L", "J", "S", "Z"] })));
assert.throws(() => bot.start(JSON.stringify({ ...initial(), board: Array.from({ length: 41 }, () => Array(10).fill(null)) })));
assert.throws(() => bot.start(JSON.stringify({ ...initial(), incoming: [{ lines: 8, active: true }] })));
assert.equal(bot.player_state_json(), before);

bot.play_with_hold_json(o, false);
assert.equal(state(bot).pieces.current, "I");
assert.equal(state(bot).pieces.hold, null);
assert.equal(bot.preview_refill_needed(), 1);
assert.throws(() => bot.think(1));
bot.new_piece("Z");
assert.deepEqual(state(bot).pieces.next, ["T", "L", "J", "S", "Z"]);
assert.equal(bot.preview_refill_needed(), 0);
bot.free();

const held = new WasmBot();
held.start(JSON.stringify(initial()));
held.play_with_hold_json(i, true);
assert.equal(state(held).pieces.hold, "O");
assert.equal(state(held).pieces.current, "T");
assert.equal(held.preview_refill_needed(), 2);
held.new_piece("Z");
held.new_piece("O");
assert.deepEqual(state(held).pieces.next, ["L", "J", "S", "Z", "O"]);
held.think(4);
assert.ok(JSON.parse(held.suggest_json()).length > 0);
held.free();

const same = new WasmBot();
same.start(JSON.stringify({ ...initial(), queue: ["O", "O", "T", "L", "J", "S"] }));
same.play_with_hold_json(o, true);
assert.equal(state(same).pieces.hold, "O");
assert.equal(state(same).pieces.current, "T");
assert.equal(same.preview_refill_needed(), 2);
const capabilities = JSON.parse(same.capabilities_json());
assert.equal(capabilities.rules_parity_verified, false);
assert.equal(capabilities.pending_garbage_in_search, false);
assert.equal(capabilities.config_profile, "h9+h12+h13-interactive");
assert.equal(capabilities.hard_node_budget, true);
assert.equal(capabilities.persistent_dag, true);
same.free();

const pending = JSON.parse(preview_garbage_one_to_one(JSON.stringify([{ lines: 8, active: false }]), 3, 0, 8));
assert.equal(pending.cancelled, 3);
assert.equal(pending.risen, 0);
assert.deepEqual(pending.remaining, [{ lines: 5, active: false }]);
const active = JSON.parse(preview_garbage_one_to_one(JSON.stringify([{ lines: 5, active: true }, { lines: 4, active: false }]), 3, 0, 8));
assert.equal(active.risen, 2);
assert.deepEqual(active.remaining, [{ lines: 4, active: false }]);
assert.throws(() => preview_garbage_one_to_one('[{"lines":3,"active":true,"hole":4}]', 0, 0, 8));

console.log(JSON.stringify({
  runtime: "Node WASM",
  status: "passed",
  nodes: nodes.toString(),
  hard_nodes: hardNodes.toString(),
  checks: [
    "search",
    "hard-node-budget",
    "pending-hard-node-budget",
    "h9+h12+h13-interactive-config",
    "empty-hold",
    "first-hold-refill-two",
    "same-piece-explicit-hold",
    "invalid-input-no-mutation",
    "no-hidden-preview",
    "normal-garbage-queue"
  ],
  capabilities
}, null, 2));
