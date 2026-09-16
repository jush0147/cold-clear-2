# Cold Clear 2

Cold Clear 2 is a modern Tetris versus bot and a complete rewrite and evolution
of [Cold Clear](https://github.com/MinusKelvin/cold-clear). It implements the
[Tetris Bot Protocol](https://github.com/tetris-bot-protocol/tbp-spec) for
interaction with a frontend, such as [Quadspace](https://github.com/SoRA-X7/Quadspace).

## Technical Features

- Column-major bitboards
- Multithreaded search on native targets
- Transposition-aware game tree
- MCTS-inspired tree expansion

## TETR.IO Tetra League Season 2 experiment

The `tetrio-s2` branch adds an experimental evaluator aimed at TETR.IO Tetra
League Season 2 rather than generic versus Tetris.

Implemented so far:

- combo state advances correctly during search
- visible B2B count is tracked instead of only a boolean B2B flag
- Tetra League Multiplier attack calculation with DOWN rounding
- B2B Charging and Surge accounting (B2B x4 starts a TL Surge of 4)
- Season 2 All Clear attack (+5) and B2B treatment
- the evaluator rewards actual calculated attack rather than the old hand-tuned
  clear-type scores
- stored Surge is represented as future value at the search horizon
- deterministic legacy-vs-S2 benchmark on identical 7-bag sequences and node
  budgets

Still intentionally not modeled yet:

- incoming garbage queues and cancellation
- the first-14-pieces double-cancel rule
- the +1 garbage-special bonus for Quads/Spins that clear garbage
- All-Mini+ immobility detection for non-T pieces / fallback Mini T-Spins
- Surge segmentation into three garbage packets
- Clutch Clears and full 1v1 timing

Run the current health-check benchmark with:

```sh
cargo run --release --bin tl_s2_bench -- --seeds 10 --pieces 200 --nodes 2000
```

It compares the original Cold Clear 2 reward model with the S2 evaluator on the
same piece sequences and search node budgets. It reports attack per piece,
Surge releases, B2B, stack height, survival, and topouts. Because there is no
incoming garbage yet, this is not a win-rate benchmark; it is meant to catch
regressions before adding a full two-player simulator.

## WebAssembly

The WASM build exposes a single-threaded `WasmBot` API intended to run inside a
browser Web Worker. The search algorithm is unchanged; JavaScript drives it in
small batches through `think()` instead of spawning a native worker thread.

Build the browser package with:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --release --target web --out-dir pkg
```

Minimal usage:

```js
import init, { WasmBot } from "./pkg/cold_clear_2.js";

await init();
const bot = new WasmBot();

bot.start(JSON.stringify(startMessage));
bot.think(100);

const moves = JSON.parse(bot.suggest_json());
const stats = JSON.parse(bot.stats_json());
```

`start()` accepts the payload of a Tetris Bot Protocol `start` message as JSON.
It may additionally contain `b2b_count` for exact TETR.IO analysis state.
`play_json()` accepts a placement as JSON, and `new_piece()` accepts one of
`I`, `O`, `T`, `L`, `J`, `S`, or `Z`. `think()` returns the number of nodes
visited as a JavaScript `BigInt` because the Rust return type is `u64`.

For browser frontends, run `WasmBot` in a Web Worker so search work cannot block
the UI thread. Search strength and device load can be controlled by choosing how
many `think()` iterations to run before requesting a suggestion.

The `wasm` GitHub Actions workflow builds both browser and Node.js packages and
runs a runtime smoke test before uploading the browser package as an artifact.

## License

Cold Clear 2 is licensed under either [Apache License Version 2.0](LICENSE-APACHE)
or [MIT License](LICENSE-MIT), at your option.
