import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { WasmBot } = require("../pkg-node/cold_clear_2.js");

const bot = new WasmBot();
const board = Array.from({ length: 40 }, () => Array(10).fill(null));

bot.start(JSON.stringify({
  board,
  queue: ["I", "O", "T", "L", "J", "S", "Z"],
  hold: null,
  combo: 0,
  back_to_back: false,
  randomizer: { type: "unknown" }
}));

const nodes = bot.think(10);
const suggestion = JSON.parse(bot.suggest_json());
const stats = JSON.parse(bot.stats_json());

if (!Number.isFinite(nodes) || nodes <= 0) {
  throw new Error(`expected search to visit nodes, got ${nodes}`);
}
if (!Array.isArray(suggestion) || suggestion.length === 0) {
  throw new Error("expected at least one suggested placement");
}
if (stats.nodes <= 0 || stats.expansions <= 0) {
  throw new Error(`unexpected stats: ${JSON.stringify(stats)}`);
}

console.log(JSON.stringify({ nodes, suggestion: suggestion[0], stats }, null, 2));
