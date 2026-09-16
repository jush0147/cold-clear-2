# Cold Clear 2

A fork of MinusKelvin's [Cold Clear 2](https://github.com/MinusKelvin/cold-clear-2)
for offline review of a player's own TETR.IO S2 position.

## Current review engine

**Working zero-gravity WASM review, actual-move comparison and concrete future
stacking plans. Not a certified optimal coach or complete TETR.IO emulator.**

Read [the review engine and integration guide](docs/review-engine.md) and
[machine-readable validation results](docs/review-quality-results.json).

`ReviewSession` is the preferred interface for review. Every legal root action,
including the actual player's move, is searched at equal lock depth and beam
widths. Explicit hold decisions, current + next-five visibility, active/pending
garbage, and own counters are modeled without PPS or soft-drop execution costs.
Hidden garbage scenarios share decisions until their observable states diverge.
Two-width sensitivity checks prevent unstable rankings from being presented as
certain player mistakes. Concrete plans include boards, attack, cancellation,
garbage rise, B2B and remaining visible pieces.

The finite horizon is 1..5 locks; the beam is approximate. Scores are heuristic,
not win probabilities. The existing S2 evaluator has not been strength-certified.
The `web/review.worker.js` adapter supports incremental progress, seek/cancel and
candidate detail requests. A complete replay-site UI is not included.

Earlier APIs remain available: `analyze_pending_json` for snapshot scenario
ranking, `WasmBot` for lower-level stateful use, and `check_replay_lock_json` for
recorded lock verification. See [the previous static-review report](docs/clock-free-review.md)
and [recorded-game audit](docs/replay-validation.md) for their historical scope.

## Build and test

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
wasm-pack build --release --target web --out-dir pkg -- --locked
cargo test --locked --lib --bins --tests
cargo test --locked --release --lib --test core_regressions --test replay_fixtures
wasm-pack build --release --target nodejs --out-dir pkg-node -- --locked
node scripts/wasm-smoke.mjs
node scripts/recorded-wasm-smoke.cjs
node scripts/review-smoke.cjs
node scripts/review-session-smoke.cjs
node scripts/test-receive-ledger.cjs
node scripts/review-worker-protocol.cjs
```

The read-only `review-quality` workflow preserves regression results and the
browser package. The latest implementation passes 53 native test functions and
the recorded/review WASM checks. The supplied recording's 3,455 first locks and
3,205 applicable boards were also rechecked locally; see the evidence guide.

The engineering duel remains KO-only, without a piece cap or attack tiebreak:

```sh
cargo run --locked --release --bin tl_s2_duel -- --seeds 20 --nodes 500
```

Its simplified model is not a Tetra League rating test. No new win-rate claim is
made by the review changes.

## License

Original Cold Clear 2 is available under Apache-2.0 or MIT. See
[LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[third-party notices](THIRD_PARTY_NOTICES.md).
