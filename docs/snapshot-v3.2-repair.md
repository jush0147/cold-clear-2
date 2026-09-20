# Snapshot v3.2 integration repair

This product repair keeps review_h9_h12 weights, current + exactly NEXT 5,
unknown bag, finite visible tail, standalone Hold and the 200,000 evaluator-node
default. It does not launch strategy experiments or implement Tetrp Phase 4B.

## Unknown observable garbage activation

`incoming[].ready_in_frames` now accepts explicit `null`: the packet is visible,
but its activation time has not been observed. Its amount and queue order remain
in the request. Known activation stays exact. A missing field still rejects.

When any packet has unknown activation, analysis crosses all ten hypothetical
hole columns with three timing hypotheses: 1 frame, frames_per_piece + 1
(capped at 600), and 600 frames. All unknown packets share the timing hypothesis
within each scenario; this is bounded sensitivity analysis, not every independent
combination or a calibrated probability distribution. The result reports the
unknown packet count and modeled delays. No hypothesis is written back as fact.
All 30 scenarios and both available root actions share ONE request node cap.

This replaces PENDING_ACTIVATION_UNKNOWN rejection. It does not read replay
future acknowledgements, hole RNG, opponent state or future attacks. Different
timing assumptions can change recommendations; no exact timing parity is claimed.

## Complete geometry with reusable landing conversion

The graph still visits the same pose/rotation-history states and produces the
same complete allowlist. Identical poses and occupied landing cells no longer
repeat CC2 coordinate conversions. Piece copies use scalar object copies rather
than structured cloning on every edge. Neither the rotation-state bound nor any
candidate set is reduced.

Local before/after comparisons cover all seven pieces with exact equality of the
entire output including state counts. Browser and non-spawn coverage remain CI
gates. These JavaScript timings exclude WASM search and are not device promises.

## Remaining work

Positive existing ARE queues and hardened/shielded/unsupported packet statuses
still reject in this revision. They require separate phase-aware handling;
ordinary unknown activation is no longer rejected. Exact frame/reset timing,
full opener/Clutch parity and empty-Hold information-gain optimization remain
unverified. Tetrp validates actual execution; recommendations never mutate the
recorded reconstruction.

The shipped manifest identifies the actual source/build commit and CI run.
