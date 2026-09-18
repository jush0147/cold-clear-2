import { performance } from "node:perf_hooks";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { WasmBot, analyze_pending_json } = require("../pkg-node/cold_clear_2.js");

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

function pendingSample(nodeBudget) {
  const request = {
    start: start(),
    incoming: [{ lines: 8, active: true }],
    pieces_placed: 30,
    garbage_sent: 0,
    frames_per_piece: 12,
    pending_delay_frames: 20,
    node_budget: nodeBudget,
  };
  const t0 = performance.now();
  const report = JSON.parse(analyze_pending_json(JSON.stringify(request)));
  const elapsed = performance.now() - t0;
  if (report.nodes > nodeBudget) throw new Error("pending analysis exceeded hard node budget");
  if (report.scenarios !== 10) throw new Error("pending analysis did not use ten scenarios");
  if (!report.candidates.length) throw new Error("pending analysis produced no candidates");
  return {
    elapsed_ms: elapsed,
    nodes: report.nodes,
    nodes_per_second: elapsed > 0 ? report.nodes * 1000 / elapsed : null,
    scenarios: report.scenarios,
    candidates: report.candidates.length,
  };
}

function summarize(nodeBudget, samples) {
  return {
    node_budget: nodeBudget,
    repeats: samples.length,
    median_ms: percentile(samples.map(x => x.elapsed_ms), 0.5),
    median_nodes_per_second: percentile(samples.map(x => x.nodes_per_second), 0.5),
    samples,
  };
}

// Warm up both exported paths before measurement.
freshSample(5000);
pendingSample(5000);

const budgets = [25000, 50000, 100000];
const repeats = 3;
const fresh = [];
const pending = [];
for (const nodeBudget of budgets) {
  fresh.push(summarize(
    nodeBudget,
    Array.from({ length: repeats }, () => freshSample(nodeBudget)),
  ));
  pending.push(summarize(
    nodeBudget,
    Array.from({ length: repeats }, () => pendingSample(nodeBudget)),
  ));
}

console.log(JSON.stringify({
  runtime: "Node WASM",
  profiles: {
    fresh_no_pending: "h9+h12+h13-interactive",
    pending_8_lines: "h9+h12-review",
  },
  workloads: {
    fresh_no_pending: {
      description: "fresh empty-board snapshot; interactive H13 profile; SevenBag speculation enabled; no pending garbage",
      results: fresh,
    },
    pending_8_lines: {
      description: "fresh empty-board snapshot; one active 8-line packet; ten hidden-hole scenarios; 12 frames/piece",
      results: pending,
    },
  },
}, null, 2));
