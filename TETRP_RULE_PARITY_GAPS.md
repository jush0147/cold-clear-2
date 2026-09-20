# Tetrp / Kiwi rule-parity and snapshot-product ledger

Updated 2026-09-20. Strategy tuning remains paused. Tetrp remains Phase 4A.
This round is a product/correctness follow-up to accepted artifact 10600729877,
workflow 35500047026, source/build
`15135ae8066c95308a3ae87ce149f5cdc7e13043`.

Read Tetrp `docs/KIWI_SNAPSHOT_PRODUCT_HANDOFF.md`. No entry here authorizes
Phase 4B, a Tetrp pin change, fresh H2, evaluator-weight changes, or a strength claim.

## Invariants which must not regress

- No replay-prefix scan, historical SevenBag recovery, piece-count modulo bag
  inference, hidden RNG/tail access, opponent future information or original-player
  future placements.
- Every external request is exactly current + NEXT5 with
  `bag_knowledge=unknown`, finite visible tail, public rules, clock/multiplier and
  already observable pending facts.
- Unknown tail is never expanded speculatively. A finite per-request search horizon
  is not a total continuation-length cap.
- Empty Hold consumes one draw and immediately refills visible NEXT5 before a new
  `hold_locked=true` request. Occupied Hold consumes no draw.
- Every lock/spawn refills from Tetrp's private isolated sequence; repeated requests
  may pass the initial six visible pieces without exposing hidden sequence state.
- Product requests are stateless and deterministic for the same allowed input.
- Default request cap is 200,000 evaluator nodes; post-Hold is a separate request.

## Snapshot-v3 repairs in this round

### Explicit same-piece Hold

Place and Hold are independent root actions. Same-type occupied Hold is not erased:
although the piece type is unchanged, Tetrp respawns/resets the active piece and
locks Hold, so the state transition is not generally identical to no-Hold.

Same-type empty Hold is also explicit and consumes NEXT[0] even when its type equals
current. The Hold action contains no landing and always requires re-analysis.

The preview revealed by an empty Hold is unknown at the pre-Hold request. Kiwi
does not infer or sample it and does not optimize its value-of-information.
The Hold branch is evaluated only through the currently known post-Hold prefix.
Capability `hold_information_gain_optimized=false` must remain false.

### Actual root geometry before Kiwi scoring

The pinned Tetrp helper enumerates the complete geometry-only current-piece landing
set from the actual active pose before Kiwi's first Place expansion. That allowlist
is applied inside root search. There is no top-K fallback; exceeding the explicit
geometry-state bound is a rejection rather than truncation.

x/y/rotation alone are insufficient. Root metadata now carries/audits hy, kick,
rotated, spin, totalRotations, resets, rotationResets, locking, forceLock, safelock,
softDropped and wall. The authority enumerator starts from the complete active Tetrp
piece, preserving SRS+ geometry/spin history.

Geometry reachability remains separate from input timing. Lock/reset timers,
handling/input state and future frame progression remain authoritative in Tetrp;
timing validation uses an isolated Engine clone. Do not claim arbitrary replay
positions are executable merely because a geometry candidate exists.

Dedicated fixtures cover non-spawn, wall, rotated, near-lock and spin-related roots.
Each result candidate is required to belong to the authority root allowlist and
the downstream helper reports its filtered candidate index.

### Stable rule/packet rejection surface

Snapshot-v3 freezes an explicit supported mechanical rule envelope in the adapter.
Values outside it reject with stable codes. Variable public attack rules continue
to be transported explicitly; Surge base/threshold, opener limit, all-clear,
garbage-special, Clutch switch, root Hold lock and attack clock/multiplier remain.

Important rejection codes include positive existing ARE queue, unknown activation,
hardened/shielded/unsupported pending status, unsupported rule values, and root
geometry/timing failures. Unknown activation is never filled with zero or guessed.

Pending that has not entered ARE remains a modeled approximation with explicit
activation timing. Positive existing ARE remains unsupported.

## Differential rule evidence

The new release workflow generates fixtures from pinned Tetrp
`0b48cb7e1a50e5f0bba6fcfee05ba8e291bebee2` and compares them against the exact
Kiwi source being built. The grid expands opener double-cancel across pending
amounts, cumulative-sent values and opener boundaries; Clutch spawn/clear rescue
cases retain topout/garbagesmash authority observations; storage-top garbage
insertion covers partial-vs-full top-row smash behavior. It also records pinned
Tetrp public-rule variants: supported variable attack-rule values must round-trip
field-for-field, while the Tetrp-visible b2bchaining=true variant must remain an
explicit unsupported Kiwi rule rather than being silently normalized.

The forecast was corrected so a partially occupied storage top row is not treated
as an immediate garbage smash; pinned Tetrp rejects insertion only at the tested
full-row boundary.

These fixtures are bounded evidence, not full parity. Keep
`rules_parity_verified=false`, full opener parity false, full Clutch parity false,
and exact ARE/bump false unless later evidence genuinely closes them.

## Remaining known limitations

- Empty-Hold unknown-preview information gain is not optimized.
- Geometry search is not a frame-accurate input-timing planner.
- Positive existing garbage ARE is rejected.
- Pending still uses the explicit review pace (normally 24F/placement), ten
  hypothetical clean-hole scenarios, integer clocking and simplified ARE/bump.
- Active-piece repair failure after an otherwise successful garbage insertion is
  not represented by the between-placement Forecast model.
- Complete terminal-reason/topout/garbage-smash classification is not certified.
- Unsupported custom rule variants reject rather than falling back to legacy values.
- Scores are evaluator heuristics, not win probabilities.
- Snapshot-v3 changes root search/compute allocation for correctness. No claim is
  made that its 200k request has the same strength as the historical speculative
  SevenBag 200k regime.

## Phase 4A downstream boundary

Phase 4A may show a landing-free Hold recommendation. Missing landing is expected.
If a post-Hold landing is desired, Tetrp must execute Hold in an isolated state,
refill the preview from its own private sequence, and send a new locked request.
Do not reuse any pre-Hold landing or infer a new one locally.

That isolated protocol test is not permission to ship user-visible continuation.
Tetrp's vendor pin is unchanged by this upstream task.

## Research validity

Pre-repair H9/H2 results remain historical/base-0-context evidence. H2 run
35491099927 remains `completed_diagnostic_base0_only`. No strategy experiment is
opened by snapshot-v3 work and `review_h9_h12` stays frozen for this artifact.
