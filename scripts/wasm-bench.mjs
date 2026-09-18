import { performance } from "node:perf_hooks";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { WasmBot } = require("../pkg-node/cold_clear_2.js");

const start = () => ({
  board: Array.from({ length: 20 }, () => Array(10).fill(null)),
  queue: ["O", "I", "T", "L", "J", "S"],
  hold: null,
  combo: 0,
  back_to_back: false,
  b2b_count: 0,
  // Six visible pieces leave Z as the deducible remainder of the opening bag.
  randomizer: { type: "seven_bag", bag_state: ["Z"] },
});

function percentile(values, p) {
  const xs = [...values].sort((a, b) => a - b);
  const i = Math.min(xs.length - 1, Math.max(0, Math.floor((xs.length - 1) * p)));
  return xs[i];
}

function freshSample(nodeBudget) {
  const bot = new WasmBot();
  bot.start(JSON.stringify(start()));
  const t0 = performance.now();
  const nodes = bot.think_nodes(nodeBudget);
  const elapsed = performance.now() - t0;
  const stats = JSON.parse(bot.stats_json());
  const suggestions = JSON.parse(bot.suggest_json());
  bot.free();
  if (nodes > BigInt(nodeBudget)) throw new Error("hard node budget exceeded");
  if (suggestions.length === 0) throw new Error("benchmark search produced no suggestion");
  return {
    elapsed_ms: elapsed,
    nodes: Number(nodes),
    nodes_per_second: elapsed > 0 ? Number(nodes) * 1000 / elapsed : null,
    max_depth: stats.max_depth,
    speculative_expansions: stats.speculative_expansions,
  };
}

// One warmup keeps module/JIT startup out of the measured fresh-snapshot samples.
freshSample(5000);

const budgets = [25000, 50000, 100000];
const repeats = 3;
const results = [];
for (const nodeBudget of budgets) {
  const samples = Array.from({ length: repeats }, () => freshSample(nodeBudget));
  results.push({
    node_budget: nodeBudget,
    repeats,
    median_ms: percentile(samples.map(x => x.elapsed_ms), 0.5),
    median_nodes_per_second: percentile(samples.map(x => x.nodes_per_second), 0.5),
    samples,
  });
}

console.log(JSON.stringify({
  runtime: "Node WASM",
  config_profile: "h9+h12-review",
  workload: "fresh empty-board snapshot; SevenBag speculation enabled; no pending garbage",
  results,
}, null, 2));
