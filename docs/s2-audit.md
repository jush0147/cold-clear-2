# S2 correctness audit

Status: experimental. Complete TETR.IO rule parity and playing strength are not
certified. See [recorded-game validation](replay-validation.md) for the current
implementation, evidence, API and limitations. That report supersedes earlier
statements that pending garbage, SRS+/180, or garbage-special scoring are absent.

The intended application is offline review of a player's own state. No opponent
board or online automatic-play connection is required or supplied.

## Evidence levels

Implemented code, internal regression tests, independent reference-model checks,
and comparison with recorded game outputs are distinct evidence levels. A green
CI check alone is not a production-game oracle.

Current independent evidence includes row-grid comparisons for bitboards,
packet-vs-per-line queue tests, and a pinned independent replay engine whose
final boards and key counters match all 20 streams of the supplied recording.
The receive counter discrepancy and unverified timing cases remain documented.

## Observation boundary

Only own board, actual current/hold, five NEXT pieces, combo/B2B, current incoming
packets and own already-observed history enter analysis. Replay RNG may be used
by the offline decoder to reproduce recorded play but never passed to the bot.
Unknown future holes and activation are explicitly modeled assumptions.

Empty-hold normalization is an internal CC2 abstraction. The public adapter
tracks actual empty hold separately. No-hold locks reveal one preview; first
empty-hold use reveals two across hold plus lock; occupied hold swaps reveal one.
Same-type current/NEXT replay playback requires an explicit hold decision.

The older WasmBot API rejects unsupported incoming fields rather than silently
ignoring them. Use analyze_pending_json for the new pending-aware forecast.
Both APIs remain spawn-based, not arbitrary mid-fall input-state solvers.

## Continuing acceptance requirements

Every rule change needs a failing regression or a versioned independent fixture,
then native and WASM checks. Keep simulator state transitions shared between
search expansion and traversal, and keep all relevant forecast state in node
keys. Preserve attack packet boundaries and visible garbage provenance.

The 3,455 successful lock checks cover this particular recording, not all legal
boards or rule interactions. Expand independent fixtures for opening cancellation,
long games, all Clutch/topout paths, packet timing, and observation visibility
before certifying general rule parity or resuming strength tuning.

KO-only duels are retained without a piece cap or attack tiebreak. Search errors
must not be mistaken for KOs. Legacy means the old evaluator on the modified
core, not untouched upstream. Earlier capped/extra-preview match results are not
valid evidence for the current implementation.
