# Cold Clear 2

Cold Clear 2 is a rewrite of [Cold Clear](https://github.com/MinusKelvin/cold-clear)
by MinusKelvin. This fork's `tetrio-s2` branch adds experimental offline replay
analysis for a player's own TETR.IO S2 position.

## Current status

**Working WASM engine and pending-aware scenario search; not full S2 parity or
strength certified.** See [the recorded-game validation report](docs/replay-validation.md)
and [correctness scope](docs/s2-audit.md).

The supplied version-19 league recording contains 3,455 locks. After SRS+/180,
surface-Mini, and garbage-special repairs, the built WASM accepts all 3,455
reconstructed placements and matches their scoring/counters. Fifty reduced
fixtures are committed and tested in native and WASM builds. Intermediate
expectations come from a pinned independent replay engine checked against actual
recorded endpoints; see the report for the remaining receive-counter mismatch.

Core features include column-major bitboards, a transposition-aware DAG,
MCTS-inspired expansion, and native worker execution. The browser build uses
cooperative `think()` calls and belongs in a Web Worker.

## WebAssembly

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
wasm-pack build --release --target web --out-dir pkg -- --locked
```

Two interfaces are available:

- `WasmBot`: stateful board/current/hold/NEXT analysis and checked replay playback.
  `player_state_json()` exposes actual player state; `preview_refill_needed()`
  handles empty hold correctly. `play_with_hold_json()` takes explicit hold use.
- `analyze_pending_json(requestJson)`: snapshot analysis including current
  pending/active garbage, own history and explicit timing assumptions. Ten
  hypothetical hole scenarios are searched; no hidden future replay state or
  opponent board is supplied. See the report for the full request schema.

`check_replay_lock_json(requestJson)` is the independent-fixture checking
interface. It evaluates one recorded placement, not a move recommendation.

Scores returned by pending analysis are heuristic scores, not win probabilities.
Hole distributions, future pace and inactive-packet delays are approximations.
Do not present the output as an exact real-time TETR.IO judgement.

## Tests

```sh
cargo test --locked --lib --bins --tests
cargo test --locked --release --test replay_fixtures
wasm-pack build --release --target nodejs --out-dir pkg-node -- --locked
node scripts/wasm-smoke.mjs
node scripts/recorded-wasm-smoke.cjs
```

The `replay-regressions` workflow repeats recorded-case checks and pending-aware
WASM search. `wasm` builds the browser package. Strength sweeps remain paused.

The engineering duel is KO-only, with no piece cap or attack-score tiebreak:

```sh
cargo run --locked --release --bin tl_s2_duel -- --seeds 20 --nodes 500
```

It still has timing/KO-model differences and is not a calibrated TL rating test.

## License

Original Cold Clear 2 is available under Apache-2.0 or MIT. See
[LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[third-party notices](THIRD_PARTY_NOTICES.md) for attributed reference material.
