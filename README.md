# Cold Clear 2

Cold Clear 2 is a rewrite of [Cold Clear](https://github.com/MinusKelvin/cold-clear)
using column-major bitboards, a transposition-aware search graph and native
worker threads. It implements the [Tetris Bot Protocol](https://github.com/tetris-bot-protocol/tbp-spec).

## TETR.IO S2 experiment: correctness audit in progress

**This branch is not yet a rule-exact TETR.IO S2 replay analyst.** Do not use its
suggestions to label a player's move as a proven mistake. Strength sweeps are
paused while observation handling, transitions and independent rule fixtures
are being audited. See [the audit and remaining gaps](docs/s2-audit.md).

The branch has a draft S2 evaluator, B2B/combo/Surge bookkeeping, normal garbage
queue primitives, a checked browser API and native/WASM regression tests.
Pending garbage is **not yet inside DAG search**. SRS+ I kicks, 180 rotation,
Clutch Clears, exact topout/timing, garbage-special +1 and opening double-cancel
are also not complete. Attack-table regressions are not independent proof of
production-game behavior.

The target input is the player's own start-of-piece position: board, current,
actual hold, five NEXT pieces and known counters/history. Opponent-board input
is out of scope. Hidden future pieces and replay RNG state are not observations.

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
  randomizer: { type: "unknown" },
}));

bot.think(100);
const moves = JSON.parse(bot.suggest_json());
const visibleState = JSON.parse(bot.player_state_json());
const capabilities = JSON.parse(bot.capabilities_json());
// capabilities.rules_parity_verified === false
// capabilities.pending_garbage_in_search === false
```

`think()` returns a JavaScript BigInt node count. `stats_json()` exposes search
statistics. An empty suggestion is not, by itself, proof of topout.

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

### Pending garbage primitives, not pending-aware advice yet

```js
import { preview_garbage_one_to_one } from "./pkg/cold_clear_2.js";
const result = JSON.parse(preview_garbage_one_to_one(
  JSON.stringify([{ lines: 8, active: false }]),
  3, // attack under the explicitly selected normal 1:1 cancellation policy
  0, // lines cleared by the placement
  8, // supplied rise cap
));
// cancelled: 3; risen: 0; remaining: [{ lines: 5, active: false }]
```

This diagnostic function does not forecast activation or insert garbage into
search boards. It does not apply opening double-cancel. Unknown fields such as
hidden hole coordinates are rejected. Passing `incoming` to `WasmBot.start()`
currently returns an unsupported-field error; it must not be silently ignored.

## Correctness checks

```sh
cargo test --lib --bins --tests
cargo test --release --test core_regressions
cargo check --all-targets
cargo check --lib --target wasm32-unknown-unknown
```

The `s2-correctness` Actions workflow retains native/release test output. The
`wasm` workflow builds browser and Node packages, then executes WASM regressions
for search, empty hold, first-hold refill, identical-piece hold decisions,
input rejection and normal garbage-queue transitions. The browser artifact is
`cold-clear-2-wasm`. Building it is not a real-browser performance benchmark.

## Diagnostic simulators

The solitaire health check has a chosen sample length, but awards no wins:

```sh
cargo run --release --bin tl_s2_bench -- --seeds 10 --pieces 200 --nodes 2000
```

The separate duel is **KO-only**, with no piece limit or attack tiebreak:

```sh
cargo run --release --bin tl_s2_duel -- --seeds 100 --nodes 500
```

Each seed is played with sides swapped. A failed search with supported legal
moves remaining is an error, not a KO. External timeouts are incomplete runs,
not manufactured wins/draws. The simplified simulator is NOT TETR.IO-conformant;
its results must not be presented as TL win rates. `legacy` and `s2` use the same
modified core with different evaluators. Only the piece stream is seeded; full
search reproducibility has not been established.

## License

Licensed under [Apache License Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
