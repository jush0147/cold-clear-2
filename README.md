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
