# Clock-free review and received-counter resolution

Tested implementation: `0d6e5aac26efd7b935cfbd220c7c5c6ff620e560`.
Validation run: https://github.com/jush0147/cold-clear-2/actions/runs/35115116810

This report supersedes the earlier unresolved receive-counter discrepancy and
the requirement to supply per-piece timing to the review API.

## Received counter: root cause, not an adjusted expected value

The pinned Triangle reference increments `stats.garbage.receive` when a
post-network-cancellation packet is admitted to the queue. The supplied version
19 recording's `stats.garbage.received` instead matches the sum of the packet's
remaining amount at its first confirmation. These are different statistics.

`scripts/receive-ledger.cjs` observes admission, local cancellation, confirmation
and tanking. It does not mutate the engine queue, board, RNG or timing. It retains
the raw admission counter and separately reports confirmation accounting.
Duplicate and late confirmation events do not count lines twice.

Across the supplied ten rounds / twenty player streams:

- Post-network admitted: 1,721 lines.
- Cancelled before confirmation: 86 lines.
- Still unconfirmed at the end: 17 lines.
- Confirmed received: 1,721 - 86 - 17 = 1,618, equal to the recording.
- All twenty received counters match; the previous fifteen discrepancies are resolved.
- All twenty endpoint boards and previously checked key counters still match.
- The complete local audit re-executed 1,996 receipt/cancel/confirm/tank ledger events.

For example, one stream admitted 141 and cancelled 9 before confirmation, so its
recorded received is 132. Another admitted 122, cancelled 10 before confirmation,
and ended with 6 unconfirmed, giving 106. No gameplay correction was necessary.

The raw player replay is not committed. The permanent CI test includes receipt
edge cases and the twenty recorded accounting summaries. Complete local event
traces can also be passed to `node scripts/test-receive-ledger.cjs trace.json`.
Those summaries alone are not advertised as full replay re-execution.

## Review objective and API

`analyze_pending_json()` defaults to zero-gravity, clock-free placement analysis.
The core movement search already enumerates grounded placements rather than
integrating falling speed. Review scoring now also disables soft-drop execution
costs. No PPS or per-piece duration is required or inferred.

```js
const result = JSON.parse(analyze_pending_json(JSON.stringify({
  start: {
    board, // bottom-up rows, G marks visible garbage provenance
    queue: [current, ...nextFive],
    hold,
    combo: consecutiveClearsBeforeMove,
    back_to_back: hasB2B,
    b2b_count: visibleB2BCount,
    randomizer: { type: 'unknown' }
  },
  incoming: [{ lines: 8, active: true }],
  pieces_placed: 30,
  garbage_sent: 20,
  iterations: 100
})));
```

Default `activation_model: "snapshot"` freezes activation at the observation:
already active packets can rise on non-clears; inactive packets remain cancelable
but do not acquire invented arrival times. This is a static decision model, not
a claim that real multiplayer garbage stops moving when gravity is zero.

An alternative `all_ready` pressure scenario treats all observed packets as ready.
Optional `fixed_pace` explicitly requires `frames_per_piece` and
`pending_delay_frames`. Pace fields are rejected without that explicit opt-in,
not silently used or ignored. Old callers must omit pace fields for static review.

The actual recording is still decoded using its recorded gravity and timing.
Changing the counterfactual review objective must not alter the historical replay.
Neither the opponent board nor hidden future pieces/RNG are accepted.

## Search improvements and checks

- Analysis uses a fixed internal search seed, unrelated to any game seed.
- Move generation has stable ordering; duplicate placements keep the minimum path cost.
- Tied report scores have a stable placement tie-break.
- With no garbage able to rise, the ten hole scenarios are equivalent: one DAG
  receives the full work budget rather than constructing ten duplicate searches.
- Different active-garbage hole scenarios remain separate.

In the previous WASM package, four repetitions of each of four recorded positions
produced different complete rankings each time; two positions changed their top
recommendation. In the new package, all four positions under both empty and
active incoming (32 runs) have exactly repeatable complete reports. This guarantee
is scoped to the same build/runtime, observation and work-count budget. It is not
a promise of identical floats across compiler versions or different platforms.

Validation: 44 native test functions passed; 27 of those were repeated in release
mode. Browser and Node WASM builds and the recorded/repeatability runtime checks
passed. The new WASM was also checked locally against all 3,455 reconstructed
locks and all 3,205 post-clear boards without garbage insertion: zero failures.

A small same-process Node 22 benchmark alternated old/new builds for sixteen runs
per build on four empty-incoming recorded positions, with 100 work iterations:

| Fixture | Old median ms | New median ms |
|---|---:|---:|
| 19 | 9.61 | 4.28 |
| 36 | 9.40 | 4.31 |
| 116 | 17.33 | 11.71 |
| 122 | 13.16 | 9.39 |

This is a small runtime measurement, NOT equal-strength or equal-node testing.
Search depth allocation and execution-cost scoring changed. No win-rate or
playing-strength gain is claimed from it.

## What is still not certified

These results support moving into controlled strategy/search optimization under
the chosen static-review objective. They do not certify all S2 edge cases. The
independent replay engine remains the source of intermediate lock expectations;
recorded endpoints provide an external check. Unknown-hole joint distributions,
per-scenario information optimism, opening-rule cases absent from this match,
exact real-time topout/Clutch, and timed network/Surge behavior remain limitations.

The received statistic was an audit-accounting mismatch, not the last possible
engine bug. Keep correctness regressions while tuning, and do not interpret a
heuristic score gap as a calibrated probability or automatically label human moves
as errors. KO-only duels retain no piece cap and no attack-score tiebreak.
