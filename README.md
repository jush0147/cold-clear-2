# Cold Clear 2

Cold Clear 2 is a rewrite of [Cold Clear](https://github.com/MinusKelvin/cold-clear)
using column-major bitboards, a transposition-aware search graph and native
worker threads. It implements the [Tetris Bot Protocol](https://github.com/tetris-bot-protocol/tbp-spec).

## TETR.IO TL S2 strategy work

**Strategy work on `s2-strategy-clean` is governed by
[`docs/strategy-experiments.md`](docs/strategy-experiments.md). Read that first before changing evaluator/search behavior or running strength experiments. For replay-review or WASM work, also read [`docs/review-bot-design.md`](docs/review-bot-design.md).**

The current goal is deliberately narrow: improve Cold Clear 2 for TETR.IO Tetra
League Season 2 and validate each strategy change with direct, paired, KO-only
bot-vs-bot matches under the same shared rules and information boundary.

The canonical strategy document defines:

- what the bot may and may not observe
- the KO-only Baseline vs Candidate protocol
- scope limits that prevent review/product work from displacing bot tuning
- how hypotheses, parameter sweeps, fresh seeds and holdout confirmation are used
- the current H1 result and the proposed next H2 experiment

Detailed machine-readable H1 evidence lives in
[`experiments/h1-pending-safety.json`](experiments/h1-pending-safety.json).

[`docs/s2-audit.md`](docs/s2-audit.md) is retained as an older correctness-audit
snapshot. It contains historical implementation-status statements that predate
later pending-aware/search work, so do not use it as the current strategy roadmap.

## WebAssembly

Build the browser package:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --release --target web --out-dir pkg
```

Run the module in a Web Worker and search in bounded batches:

```js
import init, { WasmBot } from "./pkg/cold_clear_2.js";
await init();
const bot = new WasmBot();

bot.start(JSON.stringify({
  // Bottom-up rows. Short boards are padded with empty rows ABOVE them.
  board: Array.from({ length: 20 }, () => Array(10).fill(null)),
  queue: ["O", "I", "T", "L", "J", "S"], // current + exactly five NEXT
  hold: null,
  combo: 0, // consecutive clears BEFORE this placement, not previous UI combo
  back_to_back: false,
  b2b_count: 0,
  // This opening example has seen six distinct pieces, so Z is the only
  // deducible remainder of the current SevenBag. In replay code derive this
  // state only from already-visible history, never hidden future pieces.
  randomizer: { type: "seven_bag", bag_state: ["Z"] },
}));

bot.think_nodes(50000);
const moves = JSON.parse(bot.suggest_json());
const visibleState = JSON.parse(bot.player_state_json());
const capabilities = JSON.parse(bot.capabilities_json());
```

`WasmBot` on this branch uses the interactive H9+H12+H13 profile: the scored H14 H9+H12 configuration plus the H13 persistent-DAG despeculation/backprop correctness fix. The separate pending snapshot API intentionally retains the scored H9+H12 profile because it rebuilds scenario DAGs and cannot benefit from H13 persistence. `think_nodes()` applies the same hard evaluator-node budget unit used by H14 and returns the actual searched node count as a JavaScript BigInt. `think()` remains only as a compatibility API for legacy work-iteration callers. `stats_json()` exposes search statistics. An empty suggestion is not, by itself, proof of topout.

### Browser performance benchmark

After building `pkg/`, serve the repository over HTTP and open
`benches/wasm-review.html`. The page measures fresh-snapshot 25k, 50k and
100k hard-node searches in the actual browser WASM path and reports wall time,
nodes/sec, search depth and speculative expansions. The GitHub `wasm` workflow
also uploads a ready-to-serve `cold-clear-2-browser-bench` artifact and a rough
Node-WASM benchmark JSON.

Treat Node/GitHub-runner timings only as CI evidence. Product budget decisions
should use the browser page on representative desktop hardware and, secondarily,
mobile devices.

### Correct hold and preview accounting

CC2's internal `reserve` representation is not always actual hold. Use
`player_state_json()` to inspect genuine current/hold/NEXT state. For replay
playback, include the explicit hold decision even when the piece types match:

```js
function applyRecordedMove(bot, placement, usedHold, newlyRevealed) {
  bot.play_with_hold_json(JSON.stringify(placement), usedHold);
  const required = bot.preview_refill_needed();
  if (newlyRevealed.length !== required) {
    throw new Error(`Expected ${required} newly visible previews`);
  }
  for (const piece of newlyRevealed) bot.new_piece(piece);
}
```

A normal placement reveals one preview. The first actual empty-hold use reveals
two. `new_piece()` refuses to extend a full visible queue. Searching/playing with
an incomplete queue is rejected. `play_json()` remains a compatibility method
that infers hold from type, but cannot distinguish identical-piece hold choices.
Invalid inputs return errors instead of silently corrupting the board.

## Correctness checks

```sh
cargo test --lib --bins --tests
cargo test --release --test core_regressions
cargo check --all-targets
cargo check --lib --target wasm32-unknown-unknown
```

The `s2-correctness` Actions workflow retains native/release test output. The
`wasm` workflow builds browser and Node packages, then executes WASM regressions.

## Diagnostic simulators and KO experiments

The solitaire health check remains diagnostic only:

```sh
cargo run --release --bin tl_s2_bench -- --seeds 10 --pieces 200 --nodes 2000
```

For current strategy comparisons, follow the protocol and reproduction commands
recorded in [`docs/strategy-experiments.md`](docs/strategy-experiments.md) and the
corresponding files under `experiments/`. Do not treat older duel metrics, capped
runs, APP, total attack or other surrogate metrics as proof of strategy strength.

## License

Licensed under [Apache License Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
