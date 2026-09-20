# Tetrp Phase 4A consumer task: Kiwi snapshot-v3

Use the accepted `kiwi-v1-browser` snapshot-v3 artifact only after verifying
`kiwi-build.json`, `sha256.json`, `kiwi-snapshot-acceptance.json`,
`kiwi-snapshot-browser.json`, and the rule audit evidence.

Scope is Phase 4A only. Do not vendor Rust, switch bot implementations, change
strategy weights, infer SevenBag history, or start user-visible continuation.

## Minimum integration

1. Create/fork only the current Tetrp authority state needed for analysis.
2. Call `captureSnapshotFromEngine(fork, placementTools)`.
3. Call `buildSnapshotRequest(...)`; Worker input must be exactly current+NEXT5
   plus allowlisted current facts. Never add history, bag remainder, hidden RNG/tail,
   original future placements or opponent future attacks.
4. Send the request to `kiwi-snapshot-worker.mjs`.
5. A `place` result may be displayed after authority geometry validation.
6. A `hold` result is already a complete Phase 4A recommendation. It intentionally
   has no landing. Display "建議 Hold" rather than treating the missing landing as
   an error or painting a placement from an older candidate.
7. To inspect a landing after Hold, call
   `applyHoldForReanalysis(fork, action, {Engine})`, let Tetrp reveal/refill its
   private sequence, capture the NEW current+NEXT5 snapshot, and submit a NEW
   request. The new snapshot is `hold_locked=true`.
8. Empty Hold consumes one draw; occupied Hold consumes none. Same-piece Hold is an
   independent action in both cases. Never infer Hold from piece-type mismatch.
9. A lock/spawn refills NEXT5 from Tetrp's private branch sequence. The protocol can
   repeat beyond the first six visible pieces, but this task does not authorize a
   user-visible Phase 4B loop.

## Root geometry

Snapshot-v3 enumerates the complete geometry-only current-piece landing set from
the actual Tetrp active pose before Kiwi's first Place expansion. There is no top-K
candidate truncation. Enumeration failure is an explicit rejection.

The visible root metadata includes x/y/hy/rotation/kick/rotated/spin,
totalRotations/resets/rotationResets/locking/forceLock/safelock/softDropped/wall.
This is still not a timing guarantee. Use the Tetrp authority helper for final
geometry validation and, if timing matters, the separate timing validator against
the isolated engine. Never substitute direct board painting for execution.

## Rejections and limitations

Use `normalizeSnapshotError()` and surface its stable code/message. In particular,
positive existing ARE, unknown pending activation, unsupported packet states and
unsupported public rule values are intentional rejections, not empty suggestions.

The 24F pace assumption, ten pending-hole scenarios and simplified ARE/bump remain
approximations. Empty-Hold newly revealed preview information gain is not optimized
before the Hold; the branch uses only the currently known post-Hold prefix.
Capabilities must remain truthful about these limitations and incomplete rule parity.

Cancellation means terminating the Worker. Discard stale results using the request
generation/id and dispose all analysis branch state on exit. The frozen recorded
checkpoint must not change.

This artifact does not alter the Tetrp pin automatically. Downstream adoption and
Phase 4A integration tests remain a separate Tetrp repository change.


## Revision 3.1 consumer notes

Do not reject a TL snapshot merely because `garbageare` or
`garbagearebump` is nonzero. Preserve both rule values in the request and
continue displaying the approximation warning while
`exact_are_bump_timing=false`. A positive current ARE queue is a separate
unsupported condition and has its own structured rejection.

For a 40L replay, request the explicitly labeled competitive-stacking snapshot.
Do not manufacture TL pending/clock state. Neutral combo/B2B is part of that
analysis mode, not a reconstruction claim about solo attack history.

Before vendoring the new package, reproduce the release's post-upload E2E from the
downloaded artifact. The placement helper must import successfully without any
unshipped module and must build a real pinned-Tetrp snapshot that the packaged WASM
can analyze. Do not treat manifest hashes alone as dependency-closure proof.
