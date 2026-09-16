# Recorded-game validation and pending-aware analysis

Implemented and tested source: `6fe462b4ddc6448e73d97c8ab9af35c6242e1ad9`.
Build/test run: https://github.com/jush0147/cold-clear-2/actions/runs/35111245222

## Evidence from the supplied recording

The input is a 2026-09-10 league match, replay engine version 19, containing
10 rounds, 20 player streams, and 3,455 locks. Its SHA-256 is
`1e1780dec45f2ff24188971601eaab83e367c64de795dbd1a77efde78bd019f2`.
The original replay and player identities were not committed. Fifty reduced,
anonymized reconstructed lock fixtures are retained in `tests/fixtures/`.

The recording explicitly selects SRS+, all-mini+, five NEXT pieces, 180
rotation, garbage-special bonus, charging B2B, combo blocking, and Clutch.
These are the recorded options, not a claim about every replay called S2.

Replay reconstruction uses the offline engine from MIT-licensed
`halp1/triangle@7837ee5bde8de2719472e0de3458abe6708593f5`.
On all 20 streams it matches the recorded final board, including cell types,
and pieces, lines, attack, sent, cleared, combo and B2B counters.
The `garbage.receive` bookkeeping counter differs in 15 streams and remains
unresolved. Intermediate lock expectations are reconstructed by this independent
engine, not directly logged official per-lock outputs. Matching endpoints is
supporting evidence, not a proof that every intermediate event is authoritative.

The full locally reconstructed recording was then checked against the built CC2
WASM package:

| Check | Result |
|---|---:|
| Reachable recorded locks | 3,455 / 3,455 |
| Attack packets, clear count, garbage-cleared, PC, Surge, combo/B2B | 3,455 / 3,455 |
| Post-clear boards on locks without garbage insertion | 3,205 / 3,205 |
| Reported failures | 0 |

Before the repairs, the old WASM rejected 35 locks, and the old scoring formula
under-counted 205 locks by one. The latter were garbage-special clears.
The only PC in this recording produces separate `[1, 5]` packets, total 6.
That supports additive PC +5 for this case, not a fixed total of 5.

The committed 50-case subset covers the former movement failures and selected
scoring cases. Native tests total 40 test functions; one function loops through
the 50 fixtures. The same fixture checks also run against Node WASM. These are
recorded placements and regression cases, NOT simulated matches or win-rate data.

## Implemented changes

- SRS+ I kicks, 180 rotations, and O rotation handling.
- Correct spawn coordinates and bounded Clutch-style spawn rescue.
- Surface T-Minis are not discarded by the above-stack optimization.
- Garbage-row provenance survives board parsing, insertion and line compaction.
- Garbage-special +1 is applied after the combo multiplier.
- Combo minimum and ordered Surge/ordinary/PC attack packets.
- Pending garbage is now part of hypothetical search state and its hash key.
  Both DAG traversal and child expansion use the same transition.

## Browser API

The new exported function is `analyze_pending_json(requestJson)`. It is distinct
from the older `WasmBot.start()` interface, which still does not accept incoming
packets. Run this synchronous function in a Web Worker, not on the UI thread.

```js
import init, { analyze_pending_json } from './pkg/cold_clear_2.js';
await init();
const report = JSON.parse(analyze_pending_json(JSON.stringify({
  start: {
    board, // bottom-up rows, G for already visible garbage cells
    queue: [current, ...nextFive],
    hold, // null is an actually empty hold
    combo: consecutiveClearsBeforeThisMove,
    back_to_back: hasB2B,
    b2b_count: visibleB2BCount,
    randomizer: { type: 'unknown' }
  },
  incoming: [{ lines: 8, active: true }],
  pieces_placed: 30,
  garbage_sent: 20,
  frames_per_piece: 12,
  pending_delay_frames: 20,
  iterations: 100
})));
// report.candidates contains placement, mean_score, worst_score and scenarios.
```

Inputs contain only the current observation and own history. Hidden future
pieces, replay randomizer state, hidden holes, future attacks and opponent
boards are not accepted. This is a start-of-piece counterfactual interface;
it does not authorize a second hold or undo inputs already made mid-piece.
`iterations` is a work-count budget, not milliseconds, and is shared across ten
scenarios. It must be between 10 and 10,000. At most 16 incoming packets are
supported, with 1..1,000 lines each. Invalid observations return errors.

The search actually cancels pending packets and inserts eligible remaining
lines into hypothetical boards on non-clears. It includes an opening-phase
cancellation model and garbage provenance in those branches. In a root-only
comparison on a recorded position with ten active incoming lines, the chosen
I placement changes from column 9 to column 3 when that queue is included.
This demonstrates a causal input effect, not that the recommendation is optimal.

## Deliberate approximations and remaining gaps

`rules_parity_verified` remains false. The following must not be hidden:

- Fixed frames per piece and inactive-packet delay are explicit assumptions,
  not future timings copied from the replay. They must be selected by the caller.
- Unknown holes use ten equally weighted clean-hole scenarios. These cover each
  column marginally but not every joint arrangement or actual garbage RNG.
- Per-scenario future decisions can be optimistic about information not yet
  revealed. Scores are heuristic, not calibrated win probabilities.
- Existing known incoming packets are modeled; future opponent attacks are not.
- All emitted attack packets are resolved at placement time in the forecast;
  exact Surge animation/travel timing and network cancellation are not certified.
- No actual opening double-cancel case occurs in this match. That model has
  synthetic tests only, not empirical verification from this recording.
- Clutch-style spawn rescue is implemented, but exact real-time topout, input
  lock rules and Clutch timing are not fully reproduced.
- The forecast targets this recorded S2 profile, not arbitrary replay options,
  long-game multiplier increases, bombs, or other custom modes.

Therefore this is a working pending-aware experimental analysis API, not yet a
fully certified S2 coach. Duel evaluation remains KO-only; external timeouts
are incomplete/error results and never attack-score wins.
