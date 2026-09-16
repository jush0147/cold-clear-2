# Fair, inspectable static review

Validated implementation: `0c23cc7002f1c5e24335fae61bf65643533b2c77`.
Successful native/release/WASM validation run:
https://github.com/jush0147/cold-clear-2/actions/runs/35118662614

This is an offline review engine, not an automated online player or a certified
optimal coach. The older `analyze_pending_json` API remains available; the new
`ReviewSession` is the preferred interface when comparing an actual move with
alternatives and explaining subsequent stacking.

## What changed

The new search evaluates every legal root action at the same finite lock depth
and the same beam widths. It does not give a promising root a large search budget
while grading the actual player move after a shallow expansion. Root node counts
and elapsed time can still differ; equal parameters are not equal compute.

The player move is optional, but when supplied is validated and evaluated first,
never removed by a root shortlist. A final comparison is withheld until all legal
roots have been evaluated. Partial progress is not a mistake grade.

Root actions include an explicit hold decision, including same-type current/NEXT
and genuinely empty hold. `hold_available: false` forbids another hold at the
current decision boundary; holding is available again after a lock. This does
not make arbitrary mid-fall movements or lock timers part of the model.

The search uses only the observed current + five NEXT pieces and actual hold.
The depth is limited to 1..5 locks so even an empty-hold branch does not require
a seventh piece. Future preview lists shrink in displayed plans. No new hidden
piece or replay RNG is supplied to complete a plan. This finite horizon is a
limitation, not a claim that five locks suffice for optimal S2 strategy.

Gravity and execution costs are zero. Garbage activation is frozen at the current
observation: pending packets are cancelable but do not acquire guessed arrival
times, and active packets may rise on non-clears. Own history still determines
the existing opening cancellation model. No opponent board is read.

## Garbage uncertainty and information sets

Ten hypothetical clean-hole scenarios are used when active garbage exists;
otherwise one search suffices. Scenarios with the same observable state must
choose the same action. Their plans may diverge after garbage actually rises and
the boards reveal different holes. A scenario's unobserved hole is masked from
the information-set key. This removes the old independent-scenario planner's
ability to choose an intermediate action using a hole that was not yet visible.

When no remaining packet can reveal a future hole under the static snapshot
model, revealed continuations are factored into separate searches rather than
expanding a Cartesian product. This is an optimization of the approximate search;
it is not a proof of equivalence to exhaustive search. The ten scenarios are not
all joint garbage configurations or calibrated outcome probabilities.

## Reports and concrete plans

`report_json()` includes every completed action, its explicit hold use, score,
worst scenario score, surviving scenario count, depth/width and transition count.
`candidate_json(action_id)` gives the actual simulated continuation in each
scenario, with each placement, post-step board, hold, remaining known queue,
height, holes, line clears, garbage cleared, B2B/combo, attack packets, cancelled
lines, outgoing lines, risen lines and remaining incoming.

These are search witnesses, not natural-language rationalizations or future facts.
Two beam widths are compared: requested width and max(width/4, 1). A changed
preferred root or reversed score preference is reported as `search_sensitive`.
Agreement is only a sensitivity diagnostic, not statistical confidence.

Other assessments are `no_demonstrated_improvement`,
`alternative_found_not_a_proven_mistake`, `single_width_only`, and
`survival_search_incomplete`. Failure to find a continuation after beam pruning
is not proof of forced loss. Scores are not win probabilities or automatic
blunder labels. The existing S2 hybrid evaluator is reused, with zero soft-drop
penalty and T-slot valuation limited to known remaining T pieces. No tuned
playing-strength improvement is claimed from this change.

## Browser integration

Serve `web/` beside the generated `pkg/` directory. The Worker handles `review`,
`progress`, `complete`, `details` and `cancel`, with request IDs and generation
checks so old analysis does not overwrite a newly selected replay position.
It yields between root actions. A single WASM call is not interruptible; hard
cancellation requires terminating and recreating the Worker.

```js
const worker = new Worker('./web/review.worker.js', {type: 'module'});
worker.postMessage({type: 'review', id: 'position-123', request: {
  start: {board, queue: [current, ...nextFive], hold,
    combo: consecutiveClearsBeforeMove,
    back_to_back: hasB2B, b2b_count: visibleB2B,
    randomizer: {type: 'unknown'}},
  incoming: [{lines: 8, active: true}],
  pieces_placed: 30, garbage_sent: 20,
  hold_available: true,
  actual: {placement: playerPlacement, use_hold: playerUsedHold},
  depth: 4, beam_width: 4
}});
```

The explicit values above are examples. Pass the observed garbage activation and
own historical counters rather than copying these values. Board rows are bottom-up;
G marks visible garbage provenance. Defaults are depth 4 and width 8; valid width
is 1..32. A lighter interactive preset is depth 3 / width 2, and deeper review can
use depth 5 at higher cost. Do not mix scores from different depths or widths.
The adapter must choose a start-of-piece decision boundary. Rewinding an arbitrary
mid-fall frame to spawn can suggest paths that are no longer available.

Direct WASM use is also supported:

```js
const session = new ReviewSession(JSON.stringify(request));
while (!session.step()) { /* yield between calls in an actual UI integration */ }
const report = JSON.parse(session.report_json());
const actualPlan = JSON.parse(session.candidate_json(report.comparison.actual_action_id));
const preferredPlan = JSON.parse(session.candidate_json(report.comparison.preferred_action_id));
session.free();
```

A full replay-site UI has not been integrated in this repository. Runtime tests
use Node WASM. The Worker request protocol has a separate stubbed test; that is
not an actual browser rendering or responsiveness test. Local browser testing
was blocked by the execution environment's navigation policy.

## Actual validation results

- 53 native Rust test functions passed; 49 were repeated in release mode.
- Browser and Node WASM packages built; both old API and new review runtime tests passed.
- The new review's first-step transition matches all 50 committed reconstructed fixtures.
- Local re-execution of the supplied recording passes 3,455 recorded locks using
  both the established lock checker and the new ReviewSession first-step output.
- All 3,205 applicable post-clear boards without garbage insertion match.
- The new first-step checks omit incoming to isolate movement/scoring. They are
  not full future-policy or garbage-timing equivalence tests.
- Independent replay reconstruction still matches all 20 recorded final boards
  and audited counters, including received garbage.
- Eight recorded board/queue positions, each reviewed twice at depth 4 / width 4,
  produce identical full reports. Incoming is deliberately empty and history
  inputs are controlled in this benchmark; these are not exact full replay reviews.

For fixture 19, the actual I Quad is preferred at both widths: 6 attack, height 5
and no holes after the clear. The four-lock displayed continuation keeps height 5
and no holes. For fixture 36, the preferred root changes with width, so the
comparison is explicitly search-sensitive rather than a mistake finding.

On the final CI runner, the eight empty-incoming reviews took approximately
18..125 ms each. A controlled 8-active-line case at depth 3 / width 2 took 372 ms.
A separate local Node 22 test of fixture 19 with 8 active lines, depth 4 / width 8,
went from 13,588 ms before the factorization to 2,034 ms afterwards. The longest
root step fell from 319 to 89 ms. These are small single-environment measurements,
not device guarantees, equal-strength comparisons, or live match results.

## Remaining limits

The search is finite and beam-pruned. The S2 evaluator has not been newly tuned
or independently shown to improve human review quality. Same-build deterministic
results and two-width agreement do not prove correctness of strategic judgment.
Unverified S2 timing/Clutch/topout edge cases and the synthetic-only opening-rule
coverage documented in earlier reports still apply. Exact real-time prediction
is deliberately not the objective. No piece-cap or attack tiebreak has been added
to KO-only duels, and no new win-rate result is claimed here.
