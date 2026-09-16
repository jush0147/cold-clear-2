# Cold Clear 2

Cold Clear 2 is a rewrite of [Cold Clear](https://github.com/MinusKelvin/cold-clear)
by MinusKelvin. This fork's `tetrio-s2` branch adds experimental offline review of
a player's own TETR.IO S2 position.

## Current status

**Working clock-free WASM review with pending-aware scenario search. Not complete
S2 parity or playing-strength certified.**

See [the current review report](docs/clock-free-review.md),
[the earlier recorded-game audit](docs/replay-validation.md), and
[correctness scope](docs/s2-audit.md).

The version-19 recording's 3,455 reconstructed locks and 3,205 applicable
post-clear boards pass the new WASM checks. All twenty endpoint received counters
now match after distinguishing queue admission from packet confirmation.
Intermediate expectations come from a pinned independent replay engine, not
official per-lock logs. The original player replay is not committed.

## Review API

`analyze_pending_json(requestJson)` defaults to zero-gravity placement analysis,
without PPS, soft-drop execution penalties, or guessed piece durations. It sees
only current/hold/next-five, own board and state, and currently observed incoming.
Default snapshot activation and optional alternative pressure/timing assumptions
are described in the current report. Hidden replay RNG and future events are rejected.

Same-build/runtime requests with identical observations and work budgets now
produce reproducible rankings. Equivalent no-rise hole scenarios share one search
instead of constructing ten duplicate DAGs. Scores remain heuristic, not calibrated
win probabilities or automatic mistake grades. Run synchronous analysis in a Web Worker.

`WasmBot` remains the lower-level stateful interface. `player_state_json()` exposes
actual player state; `preview_refill_needed()` handles empty hold correctly;
`play_with_hold_json()` specifies hold use. The zero-execution-cost review preset
belongs to `analyze_pending_json`, not every native/legacy configuration.
`check_replay_lock_json()` validates recorded locks rather than recommending moves.

## Build and test

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
wasm-pack build --release --target web --out-dir pkg -- --locked
cargo test --locked --lib --bins --tests
cargo test --locked --release --lib --test replay_fixtures
wasm-pack build --release --target nodejs --out-dir pkg-node -- --locked
node scripts/wasm-smoke.mjs
node scripts/recorded-wasm-smoke.cjs
node scripts/review-smoke.cjs
node scripts/test-receive-ledger.cjs
```

The read-only `review-regressions` workflow repeats native, recorded-WASM and
repeatability checks. `wasm` builds the browser package. Strategy sweeps are not
claimed to have established a strength gain.

The engineering duel remains KO-only, without piece caps or attack tiebreaks:

```sh
cargo run --locked --release --bin tl_s2_duel -- --seeds 20 --nodes 500
```

Its simplified timing/KO model is not a calibrated Tetra League rating test.

## License

Original Cold Clear 2 is available under Apache-2.0 or MIT. See
[LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[third-party notices](THIRD_PARTY_NOTICES.md).
